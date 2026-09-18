// @rustygod/plugin — thin helper over PluginService (sketch, Phase 3).
//
// Install the transport yourself: `npm i @grpc/grpc-js`
// A plugin is WASM + manifest; money crosses as integer minor units.
//
//   import { PluginClient } from './plugin.mjs';
//   const plugins = new PluginClient('127.0.0.1:50051', 'STAFF_JWT_OR_APP_TOKEN');
//   await plugins.register({ name: 'my-tax', version: '1.0.0',
//     extensionPoints: ['calculate_tax'], capabilities: [] }, wasmBytes);
//   const { taxCents } = await plugins.calculateTax(10_000n, 900n);

import grpc from '@grpc/grpc-js';
import protoLoader from '@grpc/proto-loader';
import { readFileSync } from 'node:fs';
import { fileURLToPath } from 'node:url';
import path from 'node:path';

const __dirname = path.dirname(fileURLToPath(import.meta.url));
const PROTO_DIR = path.resolve(__dirname, '../crates/proto/proto');

function loadClient(addr) {
  const def = protoLoader.loadSync(
    ['common.proto', 'plugin.proto'],
    { includeDirs: [PROTO_DIR], keepCase: false, longs: Number, defaults: true },
  );
  const pkg = grpc.loadPackageDefinition(def);
  return new pkg.rustygod.plugin.PluginService(addr, grpc.credentials.createInsecure());
}

export class PluginClient {
  constructor(addr, bearer) {
    this.client = loadClient(addr);
    this.meta = new grpc.Metadata();
    if (bearer) this.meta.add('authorization', `Bearer ${bearer}`);
  }

  call(method, req) {
    return new Promise((resolve, reject) =>
      this.client[method](req, this.meta, (err, res) =>
        err ? reject(err) : resolve(res)),
    );
  }

  async register(manifest, wasmBytes, config = {}) {
    const res = await this.call('RegisterPlugin', {
      manifest: {
        name: manifest.name,
        version: manifest.version ?? '1.0.0',
        extensionPoints: manifest.extensionPoints ?? [],
        capabilities: manifest.capabilities ?? [],
      },
      wasm: wasmBytes,
      config,
    });
    this.throwIfErrors(res);
    return res.registered;
  }

  async callPlugin(name, fn, payload) {
    const res = await this.call('CallPlugin', {
      name,
      function: fn,
      payloadJson: JSON.stringify(payload),
    });
    this.throwIfErrors(res);
    return { output: JSON.parse(res.outputJson || 'null'), events: res.events };
  }

  async calculateTax(subtotalCents, rateBps) {
    const res = await this.call('CalculateTax', {
      subtotalCents: Number(subtotalCents),
      rateBps: Number(rateBps),
    });
    this.throwIfErrors(res);
    return { taxCents: res.taxCents, totalCents: res.totalCents };
  }

  async list() {
    return (await this.call('ListPlugins', {})).plugins;
  }

  throwIfErrors(res) {
    if (res.errors?.length) {
      throw new Error(res.errors.map((e) => `${e.code}: ${e.message}`).join('; '));
    }
  }
}

// Example: node sdk/example.mjs
export async function example(addr, bearer) {
  const plugins = new PluginClient(addr, bearer);
  console.log('plugins:', (await plugins.list()).map((p) => p.name));
  console.log('tax on $100 @9%:', await plugins.calculateTax(10_000n, 900n));
}
