# sieve2workers

> Convert a Sieve program into a Cloudflare Workers for Cloudflare Email Routing.

## Install

```
cargo install sieve2workers
```

## Usage

```
sieve2workers input.sieve
```

Cloudflare Worker:
```js
import { run } from "/path/to/output.js"

export default {
  async email(message, env, ctx) {
    await run({ message });
  }
}
```
