import init, { greet } from "/pkg/rustsystem_client.js";

async function run() {
  await init();
  greet();
}

run();
