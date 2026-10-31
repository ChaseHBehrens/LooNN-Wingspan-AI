//! The LooNN network evaluated in plain Rust, so it runs without Python (and in
//! WebAssembly). A port of `LooNNModel.forward` in neural_network/loonn.py; the
//! weights come from neural_network/export_weights.py.

use std::collections::HashMap;
use std::sync::Arc;
use crate::core::{ Evaluator, Player };
use super::features::{ vectorize_player, BIRD_DIM };

const BOARD_DIM: usize = 1713;
const CARD_DIM: usize = BIRD_DIM;
const MAX_HAND: usize = 32;
const SLOT_DIM: usize = 1 + CARD_DIM + 2;
const SUPPLY_START: usize = 15 * SLOT_DIM;
const HABITATS_AT: usize = 15;
const EGGS_TO_PLAY: [f32; 6] = [0.0, 1.0, 1.0, 2.0, 2.0, 0.0];
const RELATION_DIM: usize = 11;
const DEMAND_DIM: usize = 7;
const CARD_D: usize = 64;
const HEADS: usize = 4;
const POOL_QUERIES: usize = 4;
const BOARD_D: usize = 256;
const SCORE_SCALE: f32 = 50.0;
const LAYER_NORM_EPS: f32 = 1e-5;
/// Points per bird on the mat added when choosing, as `LooNN::new` does
const BIRD_BONUS: f32 = 2.0;

/// A fully connected layer, weights stored `[out][in]` as in PyTorch
struct Linear { w: Vec<f32>, b: Vec<f32>, inputs: usize }

impl Linear {
    fn apply(&self, x: &[f32]) -> Vec<f32> {
        debug_assert_eq!(x.len(), self.inputs);
        self.b.iter().zip(self.w.chunks_exact(self.inputs))
            .map(|(b, row)| b + dot(row, x))
            .collect()
    }
}

/// Dot product summed in 8 lanes, which the compiler can turn into SIMD
fn dot(a: &[f32], b: &[f32]) -> f32 {
    let mut lanes = [0.0f32; 8];
    let (a_chunks, b_chunks) = (a.chunks_exact(8), b.chunks_exact(8));
    let tail: f32 = a_chunks.remainder().iter().zip(b_chunks.remainder()).map(|(a, b)| a * b).sum();
    for (a, b) in a_chunks.zip(b_chunks) {
        for i in 0..8 { lanes[i] += a[i] * b[i]; }
    }
    lanes.iter().sum::<f32>() + tail
}

/// A fully connected layer for mostly-zero inputs, weights stored `[in][out]`
/// so only the columns of nonzero inputs are read
struct SparseLinear { wt: Vec<f32>, b: Vec<f32> }

impl SparseLinear {
    fn apply(&self, x: &[f32]) -> Vec<f32> {
        let mut y = self.b.clone();
        let outputs = y.len();
        for (&x, column) in x.iter().zip(self.wt.chunks_exact(outputs)) {
            if x == 0.0 { continue; }
            for (y, w) in y.iter_mut().zip(column) { *y += x * w; }
        }
        y
    }
}

struct LayerNorm { w: Vec<f32>, b: Vec<f32> }

impl LayerNorm {
    fn apply(&self, x: &[f32]) -> Vec<f32> {
        let n = x.len() as f32;
        let mean = x.iter().sum::<f32>() / n;
        let var = x.iter().map(|v| (v - mean) * (v - mean)).sum::<f32>() / n;
        let scale = 1.0 / (var + LAYER_NORM_EPS).sqrt();
        x.iter().zip(&self.w).zip(&self.b).map(|((v, w), b)| (v - mean) * scale * w + b).collect()
    }
}

/// `nn.MultiheadAttention` with key padding handled by passing only real keys
struct Attention { q: Linear, k: Linear, v: Linear, out: Linear }

impl Attention {
    fn apply(&self, queries: &[Vec<f32>], keys: &[Vec<f32>]) -> Vec<Vec<f32>> {
        let head_d = CARD_D / HEADS;
        let scale = 1.0 / (head_d as f32).sqrt();
        let q: Vec<Vec<f32>> = queries.iter().map(|x| self.q.apply(x)).collect();
        let k: Vec<Vec<f32>> = keys.iter().map(|x| self.k.apply(x)).collect();
        let v: Vec<Vec<f32>> = keys.iter().map(|x| self.v.apply(x)).collect();
        q.iter().map(|q| {
            let mut mixed = vec![0.0; CARD_D];
            for h in 0..HEADS {
                let r = h * head_d..(h + 1) * head_d;
                let scores: Vec<f32> = k.iter()
                    .map(|k| q[r.clone()].iter().zip(&k[r.clone()]).map(|(a, b)| a * b).sum::<f32>() * scale)
                    .collect();
                let max = scores.iter().cloned().fold(f32::NEG_INFINITY, f32::max);
                let exp: Vec<f32> = scores.iter().map(|s| (s - max).exp()).collect();
                let total: f32 = exp.iter().sum();
                for (e, v) in exp.iter().zip(&v) {
                    for (m, v) in mixed[r.clone()].iter_mut().zip(&v[r.clone()]) { *m += e / total * v; }
                }
            }
            self.out.apply(&mixed)
        }).collect()
    }
}

/// `nn.TransformerEncoderLayer` with norm_first, GELU and no dropout
struct EncoderLayer { attention: Attention, norm1: LayerNorm, norm2: LayerNorm, linear1: Linear, linear2: Linear }

impl EncoderLayer {
    fn apply(&self, x: &mut [Vec<f32>]) {
        let normed: Vec<Vec<f32>> = x.iter().map(|t| self.norm1.apply(t)).collect();
        for (t, a) in x.iter_mut().zip(self.attention.apply(&normed, &normed)) {
            for (t, a) in t.iter_mut().zip(a) { *t += a; }
        }
        for t in x.iter_mut() {
            let mut hidden = self.linear1.apply(&self.norm2.apply(t));
            gelu(&mut hidden);
            for (t, f) in t.iter_mut().zip(self.linear2.apply(&hidden)) { *t += f; }
        }
    }
}

/// The LooNN network, predicting a player's remaining score
pub struct Network {
    board_0: SparseLinear,
    board_2: Linear,
    card_0: SparseLinear,
    card_2: Linear,
    board_to_card: Linear,
    null_card: Vec<f32>,
    encoder: Vec<EncoderLayer>,
    pool_query: Linear,
    pool: Attention,
    pool_norm: LayerNorm,
    head_0: Linear,
    head_2: Linear,
    head_4: Linear,
}

/// Tensors by name from the export_weights.py format: "LNN1", a count, then
/// for each tensor its name, shape and little-endian f32 values
struct Tensors(HashMap<String, (Vec<usize>, Vec<f32>)>);

/// Reads the weights file front to back
struct Reader<'a>(&'a [u8]);

impl<'a> Reader<'a> {
    fn take(&mut self, n: usize) -> Result<&'a [u8], String> {
        if self.0.len() < n { return Err("weights file is truncated".into()); }
        let (head, rest) = self.0.split_at(n);
        self.0 = rest;
        Ok(head)
    }

    fn u32(&mut self) -> Result<usize, String> {
        Ok(u32::from_le_bytes(self.take(4)?.try_into().unwrap()) as usize)
    }
}

impl Tensors {
    fn parse(bytes: &[u8]) -> Result<Self, String> {
        let mut r = Reader(bytes);
        if r.take(4)? != b"LNN1" { return Err("not a LooNN weights file".into()); }
        let mut tensors = HashMap::new();
        for _ in 0..r.u32()? {
            let name_len = r.u32()?;
            let name = String::from_utf8(r.take(name_len)?.to_vec()).map_err(|e| e.to_string())?;
            let shape = (0..r.u32()?).map(|_| r.u32()).collect::<Result<Vec<_>, _>>()?;
            let n: usize = shape.iter().product();
            let values = r.take(n * 4)?.chunks_exact(4).map(|c| f32::from_le_bytes(c.try_into().unwrap())).collect();
            tensors.insert(name, (shape, values));
        }
        Ok(Self(tensors))
    }

    fn get(&self, name: &str, shape: &[usize]) -> Result<Vec<f32>, String> {
        let (actual, values) = self.0.get(name).ok_or(format!("weights file has no {name}"))?;
        if actual != shape { return Err(format!("{name} has shape {actual:?}, expected {shape:?}")); }
        Ok(values.clone())
    }

    fn linear(&self, name: &str, inputs: usize, outputs: usize) -> Result<Linear, String> {
        Ok(Linear {
            w: self.get(&format!("{name}.weight"), &[outputs, inputs])?,
            b: self.get(&format!("{name}.bias"), &[outputs])?,
            inputs,
        })
    }

    fn sparse_linear(&self, name: &str, inputs: usize, outputs: usize) -> Result<SparseLinear, String> {
        let w = self.get(&format!("{name}.weight"), &[outputs, inputs])?;
        let mut wt = vec![0.0; w.len()];
        for o in 0..outputs { for i in 0..inputs { wt[i * outputs + o] = w[o * inputs + i]; } }
        Ok(SparseLinear { wt, b: self.get(&format!("{name}.bias"), &[outputs])? })
    }

    fn layer_norm(&self, name: &str, size: usize) -> Result<LayerNorm, String> {
        Ok(LayerNorm { w: self.get(&format!("{name}.weight"), &[size])?, b: self.get(&format!("{name}.bias"), &[size])? })
    }

    fn attention(&self, name: &str) -> Result<Attention, String> {
        let w = self.get(&format!("{name}.in_proj_weight"), &[3 * CARD_D, CARD_D])?;
        let b = self.get(&format!("{name}.in_proj_bias"), &[3 * CARD_D])?;
        let part = |i: usize| Linear {
            w: w[i * CARD_D * CARD_D..(i + 1) * CARD_D * CARD_D].to_vec(),
            b: b[i * CARD_D..(i + 1) * CARD_D].to_vec(),
            inputs: CARD_D,
        };
        Ok(Attention { q: part(0), k: part(1), v: part(2), out: self.linear(&format!("{name}.out_proj"), CARD_D, CARD_D)? })
    }
}

impl Network {
    /// Loads weights written by neural_network/export_weights.py
    pub fn from_bytes(bytes: &[u8]) -> Result<Self, String> {
        let t = Tensors::parse(bytes)?;
        let encoder = (0..2).map(|i| {
            let n = format!("card_encoder.layers.{i}");
            Ok(EncoderLayer {
                attention: t.attention(&format!("{n}.self_attn"))?,
                norm1: t.layer_norm(&format!("{n}.norm1"), CARD_D)?,
                norm2: t.layer_norm(&format!("{n}.norm2"), CARD_D)?,
                linear1: t.linear(&format!("{n}.linear1"), CARD_D, 2 * CARD_D)?,
                linear2: t.linear(&format!("{n}.linear2"), 2 * CARD_D, CARD_D)?,
            })
        }).collect::<Result<Vec<_>, String>>()?;
        Ok(Self {
            board_0: t.sparse_linear("board_net.0", BOARD_DIM + DEMAND_DIM, BOARD_D)?,
            board_2: t.linear("board_net.2", BOARD_D, BOARD_D)?,
            card_0: t.sparse_linear("card_embed.0", CARD_DIM + RELATION_DIM, CARD_D)?,
            card_2: t.linear("card_embed.2", CARD_D, CARD_D)?,
            board_to_card: t.linear("board_to_card", BOARD_D, CARD_D)?,
            null_card: t.get("null_card", &[1, 1, CARD_D])?,
            encoder,
            pool_query: t.linear("pool_query", BOARD_D, POOL_QUERIES * CARD_D)?,
            pool: t.attention("pool")?,
            pool_norm: t.layer_norm("pool_norm", POOL_QUERIES * CARD_D)?,
            head_0: t.linear("head.0", BOARD_D + POOL_QUERIES * CARD_D, 256)?,
            head_2: t.linear("head.2", 256, 128)?,
            head_4: t.linear("head.4", 128, 1)?,
        })
    }

    /// Predicted points the player will still score
    pub fn predict(&self, player: &Player) -> f32 {
        let (board, hand) = vectorize_player(player);
        self.forward(&board, &hand[..hand.len().min(MAX_HAND)])
    }

    fn forward(&self, board: &[f32], cards: &[Vec<f32>]) -> f32 {
        let (relations, demand) = relations(board, cards);
        let mut b = self.board_0.apply(&[board, &demand].concat());
        gelu(&mut b);
        let mut b = self.board_2.apply(&b);
        gelu(&mut b);

        let board_bias = self.board_to_card.apply(&b);
        let mut tokens = vec![self.null_card.clone()];
        for (card, rel) in cards.iter().zip(relations) {
            let mut c = self.card_0.apply(&[card.as_slice(), &rel].concat());
            gelu(&mut c);
            let mut c = self.card_2.apply(&c);
            for (c, b) in c.iter_mut().zip(&board_bias) { *c += b; }
            tokens.push(c);
        }
        for layer in &self.encoder { layer.apply(&mut tokens); }
        let queries: Vec<Vec<f32>> = self.pool_query.apply(&b).chunks_exact(CARD_D).map(|q| q.to_vec()).collect();
        let pooled = self.pool_norm.apply(&self.pool.apply(&queries, &tokens).concat());

        let mut h = self.head_0.apply(&[b.as_slice(), &pooled].concat());
        gelu(&mut h);
        let mut h = self.head_2.apply(&h);
        gelu(&mut h);
        self.head_4.apply(&h)[0] * SCORE_SCALE
    }
}

/// Per-card features relating each card to the board, and the hand's food demand.
/// A port of `relations` in loonn.py.
fn relations(board: &[f32], cards: &[Vec<f32>]) -> (Vec<Vec<f32>>, Vec<f32>) {
    let food: Vec<f32> = (0..5).map(|f| (board[SUPPLY_START + f] * 15.0).round()).collect();
    let slot = |h: usize, i: usize| &board[(h * 5 + i) * SLOT_DIM..(h * 5 + i + 1) * SLOT_DIM];
    let birds_in: Vec<usize> = (0..3).map(|h| (0..5).map(|i| slot(h, i)[0].round()).sum::<f32>() as usize).collect();
    let eggs: f32 = (0..3).flat_map(|h| (0..5).map(move |i| (h, i))).map(|(h, i)| (slot(h, i)[1 + CARD_DIM] * 6.0).round()).sum();
    let open: Vec<bool> = birds_in.iter().map(|&n| n < 5 && EGGS_TO_PLAY[n] <= eggs).collect();

    let mut demand = vec![0.0; DEMAND_DIM];
    let relations = cards.iter().map(|card| {
        let cost: Vec<f32> = (0..6).map(|f| (card[f] * 2.0).round()).collect();
        let one_of: Vec<bool> = (0..5).map(|f| card[6 + f] > 0.5).collect();
        let is_one_of = one_of.iter().any(|&o| o);
        let missing: Vec<bool> = (0..5).map(|f| cost[f] > food[f]).collect();
        let deficit: f32 = (0..5).map(|f| (cost[f] - food[f]).max(0.0)).sum();
        let surplus: f32 = (0..5).map(|f| (food[f] - cost[f]).max(0.0)).sum();
        let wild_short = (cost[5] - surplus).max(0.0);
        let one_of_short = !(0..5).any(|f| one_of[f] && food[f] > 0.0);
        let short = if is_one_of { one_of_short as u8 as f32 } else { deficit + wild_short };
        let affordable = short == 0.0;
        let closer: Vec<bool> = (0..5).map(|f| if is_one_of { one_of[f] && one_of_short } else { missing[f] || wild_short > 0.0 }).collect();
        let placeable: Vec<bool> = (0..3).map(|h| card[HABITATS_AT + h] > 0.5 && open[h]).collect();
        let playable = affordable && placeable.iter().any(|&p| p);

        for f in 0..5 { demand[f] += closer[f] as u8 as f32 / 5.0; }
        demand[5] += playable as u8 as f32 / 5.0;
        demand[6] += affordable as u8 as f32 / 5.0;
        let mut rel = Vec::with_capacity(RELATION_DIM);
        rel.push(affordable as u8 as f32);
        rel.push(short / 2.0);
        rel.extend(closer.iter().map(|&c| c as u8 as f32));
        rel.extend(placeable.iter().map(|&p| p as u8 as f32));
        rel.push(playable as u8 as f32);
        rel
    }).collect();
    (relations, demand)
}

/// Exact GELU, as `nn.GELU()`
fn gelu(x: &mut [f32]) {
    for v in x { *v = 0.5 * *v * (1.0 + erf(*v * std::f32::consts::FRAC_1_SQRT_2)); }
}

/// Error function, accurate to about 1.2e-7 (Numerical Recipes' erfc)
fn erf(x: f32) -> f32 {
    let z = (x as f64).abs();
    let t = 1.0 / (1.0 + 0.5 * z);
    let erfc = t * (-z * z - 1.26551223 + t * (1.00002368 + t * (0.37409196 + t * (0.09678418
        + t * (-0.18628806 + t * (0.27886807 + t * (-1.13520398 + t * (1.48851587
        + t * (-0.82215223 + t * 0.17087277))))))))).exp();
    (if x >= 0.0 { 1.0 - erfc } else { erfc - 1.0 }) as f32
}

/// Chooses moves with the native network, greedily, the way `LooNN::new` does
#[derive(Clone)]
pub struct NativeLooNN {
    network: Arc<Network>,
    bird_bonus: f32,
}

impl NativeLooNN {
    pub fn new(network: Arc<Network>) -> Self { Self { network, bird_bonus: BIRD_BONUS } }

    /// Predicts final scores without the per-bird bonus used when choosing
    pub fn predictor(network: Arc<Network>) -> Self { Self { network, bird_bonus: 0.0 } }
}

impl Evaluator for NativeLooNN {
    fn evaluate(&mut self, player: &Player) -> f32 {
        self.network.predict(player).max(0.0)
            + player.accumulated_score() as f32
            + self.bird_bonus * player.bird_slots().count() as f32
    }

    fn choose<T>(&mut self, options: impl Iterator<Item = (T, Player)>, _played: bool) -> Option<(T, Player)> {
        options
            .map(|(t, p)| (self.evaluate(&p), t, p))
            .max_by(|a, b| a.0.total_cmp(&b.0))
            .map(|(_, t, p)| (t, p))
    }
}
