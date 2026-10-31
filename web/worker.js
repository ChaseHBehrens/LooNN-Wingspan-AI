// Runs the WebAssembly game off the page's thread, since LooNN's turns take a
// few seconds. The page sends {type: "answer", seq, lines} or {type: "restart"}
// and gets back {type: "catalog"}, {type: "state"} and {type: "error"} messages.
import init, { Game, catalog, last_panic } from "./pkg/loonn_web.js";

let game = null;
const seed = () => Math.floor(Math.random() * 2 ** 52);

function fail(e) {
  let message = String(e);
  try { message = last_panic() || message; } catch { }
  postMessage({ type: "error", message });
}

const ready = (async () => {
  await init();
  postMessage({ type: "catalog", catalog: JSON.parse(catalog()) });
  const response = await fetch("model.bin");
  if (!response.ok) throw new Error(`Could not load model.bin (${response.status})`);
  game = new Game(new Uint8Array(await response.arrayBuffer()), seed());
  postMessage({ type: "state", state: JSON.parse(game.state()) });
})().catch(fail);

onmessage = async ({ data }) => {
  await ready;
  if (!game) return;
  try {
    const json = data.type === "restart" ? game.restart(seed()) : game.answer(data.seq, data.lines.join("\n"));
    postMessage({ type: "state", state: JSON.parse(json) });
  } catch (e) {
    fail(e);
  }
};
