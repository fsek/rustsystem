import init, { test_register, test_post } from "/pkg/rustsystem_client.js";

async function run() {
  await init();
  test_register().then((res) => {
    console.log(res.json());
  });
}

run();
