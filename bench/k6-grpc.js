// k6 load suite for rustygod-saleor (Phase 2).
// Mixed catalog workload over real gRPC, mirroring bench.rs:
// 70% ListProducts, 20% SemanticSearch, 10% GetProduct.
//
// Run:  k6 run bench/k6-grpc.js
// Env:  GRPC_ADDR (default 127.0.0.1:50051), K6_SCENARIO (load|spike, default load)
import grpc from 'k6/net/grpc';
import { check, sleep } from 'k6';
import { Trend } from 'k6/metrics';

const ADDR = __ENV.GRPC_ADDR || '127.0.0.1:50051';
const SCENARIO = __ENV.K6_SCENARIO || 'load';

const listTrend = new Trend('grpc_list_products_ms');
const searchTrend = new Trend('grpc_search_ms');
const getTrend = new Trend('grpc_get_product_ms');

const client = new grpc.Client();
client.load(['../crates/proto/proto'], 'common.proto', 'product.proto', 'ai.proto');

export const options = SCENARIO === 'spike'
  ? {
      scenarios: {
        base: { executor: 'constant-vus', vus: 20, duration: '15s' },
        spike: {
          executor: 'ramping-vus',
          startVUs: 20,
          stages: [
            { duration: '10s', target: 1000 },
            { duration: '20s', target: 1000 },
            { duration: '10s', target: 20 },
          ],
          gracefulRampDown: '5s',
        },
      },
      thresholds: {
        checks: ['rate>0.99'],
      },
    }
  : {
      vus: 200,
      duration: '30s',
      thresholds: {
        // Roadmap target: p99 < 50ms catalog reads.
        grpc_req_duration: ['p(99)<50'],
        checks: ['rate>0.999'],
      },
    };

export default function () {
  // One persistent connection per VU (real clients reuse connections;
  // connecting per iteration would benchmark TCP handshakes, not the server).
  if (!globalThis.__connected) {
    client.connect(ADDR, { plaintext: true, timeout: '5s' });
    globalThis.__connected = true;
  }
  const roll = Math.random();
  if (roll < 0.7) {
    const t0 = Date.now();
    const res = client.invoke('rustygod.product.ProductService/ListProducts', {
      channel: 'default-channel',
      first: 24,
    });
    listTrend.add(Date.now() - t0);
    check(res, { 'list ok': (r) => r && r.status === grpc.StatusOK });
  } else if (roll < 0.9) {
    const t0 = Date.now();
    const res = client.invoke('rustygod.ai.SemanticSearch/SearchProducts', {
      query: 'shirt',
      channel: 'default-channel',
      first: 10,
    });
    searchTrend.add(Date.now() - t0);
    check(res, { 'search ok': (r) => r && r.status === grpc.StatusOK });
  } else {
    const t0 = Date.now();
    const res = client.invoke('rustygod.product.ProductService/GetProduct', {
      id: '1',
    });
    getTrend.add(Date.now() - t0);
    check(res, { 'get ok': (r) => r && r.status === grpc.StatusOK });
  }
  sleep(0.01);
}
