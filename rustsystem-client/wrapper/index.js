import init, { test_register, send_vote } from "/pkg/rustsystem_client.js";

async function run() {
  await init();
  test_register().then((result) => {
    console.log(result.signature());
    console.log(result.proof());

    send_vote(result).then((response) => {
      console.log(response);
    });
  });
}

run();
