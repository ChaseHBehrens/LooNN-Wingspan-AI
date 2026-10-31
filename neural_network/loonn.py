from pathlib import Path
import json
import os
import shutil
import time
import numpy as np
import torch
import torch.nn as nn
import torch.nn.functional as F

_HERE = Path(__file__).resolve().parent
_CHECKPOINT_PATH = _HERE / "model_hand_attn.pt"
# Snapshots of the model taken during training
_SNAPSHOT_DIR = _HERE.parent / "saved_models"
_SNAPSHOT_EVERY = 2000  # player-games (one per player per game), roughly hourly
# Player-games between checkpoint saves. Saving after every one rewrote ~9 MB each
# time (terabytes over a long run); a crash now loses at most this many updates.
# _SNAPSHOT_EVERY must be a multiple of this.
_SAVE_EVERY = 20

# Replay data: fixed-size ring buffers, so disk use stays bounded. Boards are
# stored as float16. Hands are stored as indices into a table of unique card
# vectors, so a position costs a few bytes for its hand instead of
# MAX_HAND * CARD_DIM floats.
_BOARDS_PATH = _HERE / "boards.bin"
_HANDS_PATH = _HERE / "hands.bin"
_TARGETS_PATH = _HERE / "targets.bin"
_CARDS_PATH = _HERE / "cards.npy"
_META_PATH = _HERE / "replay_meta.json"

_ARCH = "hand_attn_v3"
# The replay buffer stores raw board and card vectors, which v3 still uses
_REPLAY_ARCH = "hand_attn_v2"
_BOARD_DIM = 1713
_CARD_DIM = 108
_MAX_HAND = 32

# Layout of the vectors built in src/evaluators/loonn.rs
_SLOT_DIM = 1 + _CARD_DIM + 2  # occupied, bird, eggs / 6, room for eggs / 6
_SUPPLY_START = 15 * _SLOT_DIM  # 5 food counts / 15
_CARD_COST_ALL = slice(0, 6)  # 5 foods then wild, each count / 2
_CARD_COST_ONE_OF = slice(6, 11)  # flags for the 5 foods
_CARD_HABITATS = slice(15, 18)  # forest, grassland, wetland
_EGGS_TO_PLAY = [0, 1, 1, 2, 2]  # by birds already in the habitat

# Computed from the board and each card, so the network does not have to learn
# to compare costs with the food supply: affordable, food short / 2, whether
# gaining each food gets closer to affording it, can be placed in each habitat,
# playable now
_RELATION_DIM = 11
# Over the hand: cards each food gets closer to affording, playable cards,
# affordable cards (each / 5)
_DEMAND_DIM = 7

_CARD_D = 64
_CARD_HEADS = 4
_CARD_LAYERS = 2
_POOL_QUERIES = 4
_BOARD_D = 256
_SCORE_SCALE = 50.0

# Halved from 3e-4 at ~79k player-games, once progress slowed to ~1 point per 10k
_LEARNING_RATE = 1.5e-4
# Each position is seen 2x when new, then sampled REPLAY_SAMPLE_SIZE / REPLAY_CAPACITY
# of the time per player-game while it stays in the replay buffer
_EPOCHS_PER_GAME = 1
_BATCH_SIZE = 128
_GRAD_CLIP_NORM = 1.0

_REPLAY_SAMPLE_SIZE = 2048
# ~40 minutes of 4 player play at 8 games in parallel; ~1 GB on disk
_REPLAY_CAPACITY = 300_000
_NEW_DATA_OVERSAMPLE = 2
_GAMMA = 1.0
# Bootstrapped targets follow the greedy policy as it improves, where Monte Carlo
# returns only describe the noisy play that produced them. Targets are computed
# once and stored, so the replay window is kept short to limit staleness.
_TD_LAMBDA = 0.8

_TARGET_SYNC_EVERY = 20

_device: torch.device
_model: "LooNNModel"
_target_model: "LooNNModel"
_optimizer: torch.optim.Optimizer
_train_call_count: int
_card_table: np.ndarray
_card_ids: dict[bytes, int]


def relations(board: torch.Tensor, cards: torch.Tensor, pad: torch.Tensor) -> tuple[torch.Tensor, torch.Tensor]:
    """Per-card features relating each card to the board, [B, N, RELATION_DIM], and
    the hand's food demand, [B, DEMAND_DIM]. Mirrors the rules in src/core/player.rs
    (playable_birds_in_habitat), which ignore paying 2 foods as a wild."""
    with torch.no_grad():
        batch = board.shape[0]
        supply = torch.round(board[:, _SUPPLY_START : _SUPPLY_START + 5] * 15)
        slots = board[:, : _SUPPLY_START].reshape(batch, 3, 5, _SLOT_DIM)
        birds_in_habitat = torch.round(slots[..., 0]).sum(-1)
        eggs = torch.round(slots[..., 1 + _CARD_DIM] * 6).sum((1, 2))

        cost = torch.round(cards[..., _CARD_COST_ALL] * 2)
        one_of = cards[..., _CARD_COST_ONE_OF] > 0.5
        is_one_of = one_of.any(-1)

        food = supply.unsqueeze(1)
        missing = cost[..., :5] > food
        deficit = (cost[..., :5] - food).clamp(min=0).sum(-1)
        surplus = (food - cost[..., :5]).clamp(min=0).sum(-1)
        wild_short = (cost[..., 5] - surplus).clamp(min=0)
        one_of_short = ~(one_of & (food > 0)).any(-1)
        short = torch.where(is_one_of, one_of_short.float(), deficit + wild_short)
        affordable = short == 0
        # Gaining a food the cost needs fills a deficit; any other food adds to
        # the surplus, which helps only if a wild is still unpaid
        closer = torch.where(
            is_one_of.unsqueeze(-1),
            one_of & one_of_short.unsqueeze(-1),
            missing | (wild_short > 0).unsqueeze(-1),
        )
        eggs_needed = torch.tensor(_EGGS_TO_PLAY + [0], dtype=eggs.dtype, device=board.device)
        open_habitat = (birds_in_habitat < 5) & (eggs_needed[birds_in_habitat.long()] <= eggs.unsqueeze(1))
        placeable = (cards[..., _CARD_HABITATS] > 0.5) & open_habitat.unsqueeze(1)
        playable = affordable & placeable.any(-1)

        rel = torch.cat([
            affordable.unsqueeze(-1).float(),
            (short / 2).unsqueeze(-1),
            closer.float(),
            placeable.float(),
            playable.unsqueeze(-1).float(),
        ], -1)
        in_hand = (~pad).float()
        demand = torch.cat([
            (closer.float() * in_hand.unsqueeze(-1)).sum(1),
            (playable.float() * in_hand).sum(1, keepdim=True),
            (affordable.float() * in_hand).sum(1, keepdim=True),
        ], -1) / 5
        return rel * in_hand.unsqueeze(-1), demand


class LooNNModel(nn.Module):
    def __init__(self):
        super().__init__()
        self.board_net = nn.Sequential(
            nn.Linear(_BOARD_DIM + _DEMAND_DIM, _BOARD_D),
            nn.GELU(),
            nn.Linear(_BOARD_D, _BOARD_D),
            nn.GELU(),
        )
        self.card_embed = nn.Sequential(
            nn.Linear(_CARD_DIM + _RELATION_DIM, _CARD_D),
            nn.GELU(),
            nn.Linear(_CARD_D, _CARD_D),
        )
        # Lets each card's embedding depend on the board (food, open slots, ...)
        self.board_to_card = nn.Linear(_BOARD_D, _CARD_D)
        # Always-present token so empty hands still have something to attend to
        self.null_card = nn.Parameter(torch.zeros(1, 1, _CARD_D))
        self.card_encoder = nn.TransformerEncoder(
            nn.TransformerEncoderLayer(
                _CARD_D,
                _CARD_HEADS,
                dim_feedforward=2 * _CARD_D,
                dropout=0.0,
                activation="gelu",
                batch_first=True,
                norm_first=True,
            ),
            _CARD_LAYERS,
            enable_nested_tensor=False,
        )
        # Board-conditioned queries that pool the hand into a fixed-size vector
        self.pool_query = nn.Linear(_BOARD_D, _POOL_QUERIES * _CARD_D)
        self.pool = nn.MultiheadAttention(_CARD_D, _CARD_HEADS, batch_first=True)
        self.pool_norm = nn.LayerNorm(_POOL_QUERIES * _CARD_D)
        self.head = nn.Sequential(
            nn.Linear(_BOARD_D + _POOL_QUERIES * _CARD_D, 256),
            nn.GELU(),
            nn.Linear(256, 128),
            nn.GELU(),
            nn.Linear(128, 1),
        )

    def hand_vector(self, b: torch.Tensor, cards: torch.Tensor, pad: torch.Tensor) -> torch.Tensor:
        """cards: [B, N, CARD_DIM + RELATION_DIM], pad: [B, N] (True = padding) -> [B, Q * CARD_D]"""
        batch = cards.shape[0]
        c = self.card_embed(cards) + self.board_to_card(b).unsqueeze(1)
        c = torch.cat([self.null_card.expand(batch, 1, _CARD_D), c], dim=1)
        pad = torch.cat([torch.zeros(batch, 1, dtype=torch.bool, device=pad.device), pad], dim=1)
        c = self.card_encoder(c, src_key_padding_mask=pad)
        q = self.pool_query(b).view(batch, _POOL_QUERIES, _CARD_D)
        h, _ = self.pool(q, c, c, key_padding_mask=pad, need_weights=False)
        return self.pool_norm(h.reshape(batch, _POOL_QUERIES * _CARD_D))

    def forward(self, board: torch.Tensor, cards: torch.Tensor, pad: torch.Tensor) -> torch.Tensor:
        rel, demand = relations(board, cards, pad)
        b = self.board_net(torch.cat([board, demand], -1))
        h = self.hand_vector(b, torch.cat([cards, rel], -1), pad)
        return self.head(torch.cat([b, h], dim=-1)).squeeze(-1) * _SCORE_SCALE


def init() -> None:
    global _model, _target_model, _optimizer, _device, _train_call_count
    _device = torch.device("cuda" if torch.cuda.is_available() else "cpu")
    _model = LooNNModel().to(_device)
    _target_model = LooNNModel().to(_device)
    _optimizer = torch.optim.AdamW(_model.parameters(), lr=_LEARNING_RATE)
    _train_call_count = 0
    _load_card_table()
    if _CHECKPOINT_PATH.exists():
        try:
            checkpoint = torch.load(_CHECKPOINT_PATH, map_location=_device)
            same_inputs = (
                checkpoint.get("board_dim") == _BOARD_DIM
                and checkpoint.get("card_dim") == _CARD_DIM
            )
            if same_inputs and checkpoint.get("arch") == _ARCH:
                _model.load_state_dict(checkpoint["model_state"])
                _optimizer.load_state_dict(checkpoint["optimizer_state"])
                # The saved state carries the learning rate it was saved with
                for group in _optimizer.param_groups:
                    group["lr"] = _LEARNING_RATE
                _train_call_count = checkpoint.get("train_call_count", 0)
            elif same_inputs and checkpoint.get("arch") == "hand_attn_v2":
                # The optimizer starts fresh, since its state has the old shapes
                _model.load_state_dict(_migrate_from_v2(checkpoint["model_state"]))
                _train_call_count = checkpoint.get("train_call_count", 0)
                print("[loonn] warm started from the hand_attn_v2 checkpoint")
            else:
                print("[loonn] checkpoint does not match current architecture; starting fresh")
        except Exception as e:
            print(f"[loonn] failed to load checkpoint, starting fresh: {e}")
    _target_model.load_state_dict(_model.state_dict())


def _migrate_from_v2(old: dict) -> dict:
    """Maps hand_attn_v2 weights onto v3 so the model predicts exactly what it did.
    The new inputs get zero weights, and the 2 new pool queries copy the old 2:
    the duplicated pooled vector normalizes the same way, and the head gives
    the copies zero weight."""
    new = dict(old)

    def zero_columns(w: torch.Tensor, extra: int) -> torch.Tensor:
        return torch.cat([w, w.new_zeros(w.shape[0], extra)], dim=1)

    new["board_net.0.weight"] = zero_columns(old["board_net.0.weight"], _DEMAND_DIM)
    new["card_embed.0.weight"] = zero_columns(old["card_embed.0.weight"], _RELATION_DIM)
    copies = _POOL_QUERIES // 2
    for key in ("pool_query.weight", "pool_query.bias", "pool_norm.weight", "pool_norm.bias"):
        new[key] = old[key].repeat(copies, *[1] * (old[key].dim() - 1))
    new["head.0.weight"] = zero_columns(old["head.0.weight"], (_POOL_QUERIES - 2) * _CARD_D)
    return new


def _replace_atomically(path: Path, write) -> None:
    """Writes via a temporary file and renames it into place, so other
    processes (like the test binary) never read a half-written file."""
    tmp = path.with_name(path.name + ".tmp")
    with open(tmp, "wb") as f:
        write(f)
    # On Windows the rename fails while another process (antivirus, the search
    # indexer, the test binary) has the file open, so retry briefly
    for attempt in range(20):
        try:
            os.replace(tmp, path)
            return
        except PermissionError:
            if attempt == 19:
                raise
            time.sleep(0.25)


def _save_checkpoint() -> None:
    _replace_atomically(_CHECKPOINT_PATH, lambda f: torch.save(
        {
            "arch": _ARCH,
            "board_dim": _BOARD_DIM,
            "card_dim": _CARD_DIM,
            "model_state": _model.state_dict(),
            "optimizer_state": _optimizer.state_dict(),
            "train_call_count": _train_call_count,
        },
        f,
    ))


def _pad_hands(hands: list) -> tuple[torch.Tensor, torch.Tensor]:
    """Pads a batch of variable-length hands to [B, N, CARD_DIM] plus a padding mask."""
    n = max(1, min(_MAX_HAND, max((len(h) for h in hands), default=0)))
    cards = np.zeros((len(hands), n, _CARD_DIM), dtype=np.float32)
    pad = np.ones((len(hands), n), dtype=bool)
    for i, hand in enumerate(hands):
        hand = hand[:_MAX_HAND]
        if len(hand):
            cards[i, : len(hand)] = np.asarray(hand, dtype=np.float32)
            pad[i, : len(hand)] = False
    return torch.from_numpy(cards).to(_device), torch.from_numpy(pad).to(_device)


def _unpack(boards: bytes, cards: bytes, hand_lens: list[int]) -> tuple[np.ndarray, list[np.ndarray]]:
    """Packed little-endian f32 boards and cards from Rust -> boards [B, BOARD_DIM] and one [n, CARD_DIM] array per hand"""
    boards_arr = np.frombuffer(boards, dtype="<f4").reshape(-1, _BOARD_DIM)
    cards_arr = np.frombuffer(cards, dtype="<f4").reshape(-1, _CARD_DIM)
    assert len(hand_lens) == len(boards_arr), f"{len(hand_lens)} hands for {len(boards_arr)} boards"
    return boards_arr, np.split(cards_arr, np.cumsum(hand_lens)[:-1])


def _predict(model: LooNNModel, boards: np.ndarray, hands: list[np.ndarray]) -> np.ndarray:
    model.eval()
    with torch.inference_mode():
        b = torch.from_numpy(np.ascontiguousarray(boards, dtype=np.float32)).to(_device)
        cards, pad = _pad_hands(hands)
        return model(b, cards, pad).cpu().numpy()


def evaluate_packed(boards: bytes, cards: bytes, hand_lens: list[int]) -> list[float]:
    return _predict(_model, *_unpack(boards, cards, hand_lens)).tolist()


def _compute_td_lambda_targets(
    boards: np.ndarray, hands: list[np.ndarray], rewards: np.ndarray
) -> np.ndarray:
    T = len(boards)
    targets = np.zeros(T, dtype=np.float32)
    targets[T - 1] = rewards[T - 1]
    if T == 1:
        return targets

    next_values = np.zeros(T, dtype=np.float32)
    if _TD_LAMBDA < 1.0:
        next_values[: T - 1] = _predict(_target_model, boards[1:], hands[1:])

    for t in range(T - 2, -1, -1):
        bootstrap = (1.0 - _TD_LAMBDA) * next_values[t] + _TD_LAMBDA * targets[t + 1]
        targets[t] = rewards[t] + _GAMMA * bootstrap
    return targets


# ---- Card table -------------------------------------------------------------

def _load_card_table() -> None:
    global _card_table, _card_ids
    if _CARDS_PATH.exists():
        _card_table = np.load(_CARDS_PATH)
    else:
        # Row 0 is the padding card
        _card_table = np.zeros((1, _CARD_DIM), dtype=np.float32)
    _card_ids = {row.tobytes(): i for i, row in enumerate(_card_table) if i > 0}


def _card_indices(hand: np.ndarray) -> np.ndarray:
    global _card_table
    idx = np.zeros(_MAX_HAND, dtype=np.int16)
    new_rows = []
    for i, card in enumerate(hand[:_MAX_HAND]):
        row = np.asarray(card, dtype=np.float32)
        key = row.tobytes()
        card_id = _card_ids.get(key)
        if card_id is None:
            card_id = len(_card_table) + len(new_rows)
            _card_ids[key] = card_id
            new_rows.append(row)
        idx[i] = card_id
    if new_rows:
        _card_table = np.concatenate([_card_table, np.stack(new_rows)], axis=0)
        _replace_atomically(_CARDS_PATH, lambda f: np.save(f, _card_table))
    return idx


def _cards_from_indices(idx: np.ndarray) -> tuple[torch.Tensor, torch.Tensor]:
    table = torch.from_numpy(_card_table).to(_device)
    idx_t = torch.from_numpy(idx.astype(np.int64)).to(_device)
    return table[idx_t], idx_t == 0


# ---- Replay buffer ----------------------------------------------------------

_replay: tuple[np.memmap, np.memmap, np.memmap] | None = None
_replay_count: int = 0  # positions ever written; slot = count % capacity


def _open_replay() -> tuple[np.memmap, np.memmap, np.memmap]:
    """Opens the replay buffers, creating them if they are missing or from another architecture."""
    global _replay, _replay_count
    if _replay is not None:
        return _replay
    meta = {}
    if _META_PATH.exists():
        try:
            meta = json.loads(_META_PATH.read_text())
        except (ValueError, OSError):
            meta = {}
    fresh = (
        meta.get("arch") != _REPLAY_ARCH
        or meta.get("capacity") != _REPLAY_CAPACITY
        or not all(p.exists() for p in (_BOARDS_PATH, _HANDS_PATH, _TARGETS_PATH))
    )
    if fresh and _META_PATH.exists():
        print("[loonn] replay buffer does not match current architecture; starting fresh")
    mode = "w+" if fresh else "r+"
    # After a crash, Windows (antivirus, or the dying process's handles) can keep
    # these files locked for minutes, so wait instead of crashing again
    for attempt in range(60):
        try:
            _replay = (
                np.memmap(_BOARDS_PATH, dtype=np.float16, mode=mode, shape=(_REPLAY_CAPACITY, _BOARD_DIM)),
                np.memmap(_HANDS_PATH, dtype=np.int16, mode=mode, shape=(_REPLAY_CAPACITY, _MAX_HAND)),
                np.memmap(_TARGETS_PATH, dtype=np.float32, mode=mode, shape=(_REPLAY_CAPACITY,)),
            )
            break
        except PermissionError:
            if attempt == 59:
                raise
            print("[loonn] replay files are locked; retrying in 10s")
            time.sleep(10)
    _replay_count = 0 if fresh else int(meta.get("count", 0))
    if fresh:
        _write_meta()
    return _replay


def _write_meta() -> None:
    _META_PATH.write_text(json.dumps({"arch": _REPLAY_ARCH, "capacity": _REPLAY_CAPACITY, "count": _replay_count}))


def _append_positions(boards: np.ndarray, hands: np.ndarray, targets: np.ndarray) -> None:
    global _replay_count
    replay_b, replay_h, replay_y = _open_replay()
    idx = (_replay_count + np.arange(len(boards))) % _REPLAY_CAPACITY
    replay_b[idx] = boards.astype(np.float16)
    replay_h[idx] = hands
    replay_y[idx] = targets.astype(np.float32)
    for mm in (replay_b, replay_h, replay_y):
        mm.flush()
    _replay_count += len(boards)
    _write_meta()


def _sample_replay(sample_size: int) -> tuple[np.ndarray, np.ndarray, np.ndarray] | None:
    replay_b, replay_h, replay_y = _open_replay()
    valid_count = min(_replay_count, _REPLAY_CAPACITY)
    if valid_count == 0:
        return None
    idx = np.random.choice(valid_count, size=min(sample_size, valid_count), replace=False)
    idx.sort()
    return np.array(replay_b[idx], dtype=np.float32), np.array(replay_h[idx]), np.array(replay_y[idx])


def sample_states(count: int) -> list[tuple[list[float], list[list[float]]]]:
    """Random (board, hand) positions from the replay buffer, for analysis tools."""
    sample = _sample_replay(count)
    if sample is None:
        return []
    boards, hands, _ = sample
    return [
        (board.tolist(), [_card_table[i].tolist() for i in idx if i != 0])
        for board, idx in zip(boards, hands)
    ]


# ---- Training ---------------------------------------------------------------

def _rewards_from_return_to_go(scores: list[float]) -> np.ndarray:
    scores_arr = np.asarray(scores, dtype=np.float32)
    rewards = np.empty_like(scores_arr)
    rewards[:-1] = scores_arr[:-1] - scores_arr[1:]
    rewards[-1] = scores_arr[-1]
    return rewards


def train(boards: bytes, cards: bytes, hand_lens: list[int], scores: list[float]) -> None:
    global _train_call_count
    if len(hand_lens) == 0:
        return
    boards, hands = _unpack(boards, cards, hand_lens)

    rewards = _rewards_from_return_to_go(scores)
    new_y = _compute_td_lambda_targets(boards, hands, rewards)

    new_b = np.array(boards, dtype=np.float32)
    # Everything goes through the card table so new and replayed hands are batched alike
    new_h = np.stack([_card_indices(h) for h in hands])

    replay = _sample_replay(_REPLAY_SAMPLE_SIZE)
    _append_positions(new_b, new_h, new_y)

    if _NEW_DATA_OVERSAMPLE > 1:
        new_b = np.tile(new_b, (_NEW_DATA_OVERSAMPLE, 1))
        new_h = np.tile(new_h, (_NEW_DATA_OVERSAMPLE, 1))
        new_y = np.tile(new_y, _NEW_DATA_OVERSAMPLE)

    if replay is not None:
        replay_b, replay_h, replay_y = replay
        train_b = np.concatenate([new_b, replay_b], axis=0)
        train_h = np.concatenate([new_h, replay_h], axis=0)
        train_y = np.concatenate([new_y, replay_y], axis=0)
    else:
        train_b, train_h, train_y = new_b, new_h, new_y

    b = torch.from_numpy(train_b).to(_device)
    cards, pad = _cards_from_indices(train_h)
    y = torch.from_numpy(train_y).to(_device)

    _model.train()
    n = b.shape[0]
    for epoch in range(_EPOCHS_PER_GAME):
        perm = torch.randperm(n, device=_device)
        epoch_loss_sum = 0.0
        num_batches = 0
        for start in range(0, n, _BATCH_SIZE):
            idx = perm[start : start + _BATCH_SIZE]
            batch_pad = pad[idx]
            # Trim padding columns that are empty for the whole batch
            width = max(1, int((~batch_pad).sum(dim=1).max().item()))
            _optimizer.zero_grad()
            predictions = _model(b[idx], cards[idx, :width], batch_pad[:, :width])
            loss = F.mse_loss(predictions / _SCORE_SCALE, y[idx] / _SCORE_SCALE)
            loss.backward()
            torch.nn.utils.clip_grad_norm_(_model.parameters(), _GRAD_CLIP_NORM)
            _optimizer.step()
            epoch_loss_sum += loss.item()
            num_batches += 1
        # Reported in squared points
        avg_epoch_loss = epoch_loss_sum / num_batches * _SCORE_SCALE ** 2
        print(f"[loonn] epoch {epoch + 1}/{_EPOCHS_PER_GAME}  n={n}  mse_loss={avg_epoch_loss:.4f}")

    _train_call_count += 1
    if _train_call_count % _SAVE_EVERY == 0:
        _save_checkpoint()
    if _train_call_count % _SNAPSHOT_EVERY == 0:
        _SNAPSHOT_DIR.mkdir(exist_ok=True)
        snapshot = _SNAPSHOT_DIR / f"model_hand_attn_{_train_call_count:08d}_player_games.pt"
        shutil.copyfile(_CHECKPOINT_PATH, snapshot)
        print(f"[loonn] saved snapshot {snapshot.name}")
    if _train_call_count % _TARGET_SYNC_EVERY == 0:
        _target_model.load_state_dict(_model.state_dict())


def saliency(board: list[float], hand: list[list[float]]) -> list[float]:
    """Gradient * input for each board feature, followed by each card feature summed over the hand."""
    _model.eval()
    b = torch.tensor([board], dtype=torch.float32, device=_device, requires_grad=True)
    cards, pad = _pad_hands([hand])
    cards.requires_grad_(True)
    prediction = _model(b, cards, pad)
    _model.zero_grad()
    prediction.backward()
    board_contribution = (b.grad * b).squeeze(0)
    card_contribution = (cards.grad * cards).masked_fill(pad.unsqueeze(-1), 0.0).sum(dim=1).squeeze(0)
    return torch.cat([board_contribution, card_contribution]).detach().cpu().tolist()
