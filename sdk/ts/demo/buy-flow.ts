// Buy-flow demo over the generated clients (Phase 4):
// search → list → create checkout → add lines → complete → order.
// Run against a local server:  npm run demo   (needs :50051 + DB)
//
// Uses only the generated ts-proto clients — the same modules a storefront
// ships. No hand-written request shapes.
import { credentials, type ServiceError } from '@grpc/grpc-js';
import { SemanticSearchClient, type SearchProductsResponse } from '../src/ai.js';
import {
  CheckoutServiceClient,
  type AddLinesResponse,
  type CompleteCheckoutResponse,
  type CreateCheckoutResponse,
} from '../src/checkout.js';
import { OrderServiceClient, type GetOrderResponse } from '../src/order.js';
import { ProductServiceClient, type ListProductsResponse } from '../src/product.js';

const ADDR = process.env.GRPC_ADDR ?? '127.0.0.1:50051';
const creds = credentials.createInsecure();

const search = new SemanticSearchClient(ADDR, creds);
const products = new ProductServiceClient(ADDR, creds);
const checkoutSvc = new CheckoutServiceClient(ADDR, creds);
const orders = new OrderServiceClient(ADDR, creds);

type Cb<T> = (err: ServiceError | null, res: T) => void;

function call<T>(fn: (cb: Cb<T>) => void): Promise<T> {
  return new Promise((resolve, reject) =>
    fn((err, res) => (err ? reject(err) : resolve(res))),
  );
}

function failIfErrors(res: { errors?: { code: string; message: string }[] }, what: string) {
  if (res.errors?.length) {
    throw new Error(`${what}: ${res.errors.map((e) => `${e.code} ${e.message}`).join('; ')}`);
  }
}

async function main() {
  // 1. Search + list the catalog.
  const hits: SearchProductsResponse = await call((cb) =>
    search.searchProducts({ query: 'shirt', channel: 'default-channel', first: 10 }, cb),
  );
  console.log(`search: ${hits.results.length} hits`);

  const catalog: ListProductsResponse = await call((cb) =>
    products.listProducts(
      { channel: 'default-channel', first: 5, after: '', categoryId: '' },
      cb,
    ),
  );
  const first = catalog.products[0];
  const variantId = first.variants[0].id;
  console.log(`catalog: ${catalog.products.length} products, buying variant ${variantId}`);

  // 2. Checkout lifecycle.
  const created: CreateCheckoutResponse = await call((cb) =>
    checkoutSvc.createCheckout({ channel: 'default-channel', email: 'demo@example.com' }, cb),
  );
  failIfErrors(created, 'create');
  const coId = created.checkout!.id;

  const withLines: AddLinesResponse = await call((cb) =>
    checkoutSvc.addLines({
      checkoutId: coId,
      lines: [{ variantId, quantity: 2, unitPrice: undefined, totalPrice: undefined }],
    }, cb),
  );
  failIfErrors(withLines, 'add-lines');
  console.log(`checkout ${coId}: total ${withLines.checkout!.total!.amount}`);

  const done: CompleteCheckoutResponse = await call((cb) =>
    checkoutSvc.completeCheckout({ checkoutId: coId }, cb),
  );
  failIfErrors(done, 'complete');

  // 3. Read it back.
  const got: GetOrderResponse = await call((cb) => orders.getOrder({ id: done.orderId }, cb));
  console.log(`order ${got.order!.number} minted, total ${got.order!.total!.amount}`);
  console.log(`order status: ${got.order!.status}, lines: ${got.order!.lines.length}`);
  console.log('BUY FLOW OK');
}

main().catch((e) => {
  console.error('BUY FLOW FAILED:', e.message);
  process.exit(1);
});
