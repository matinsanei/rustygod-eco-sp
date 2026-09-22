import http from "k6/http";
import { check } from "k6";

const URL = __ENV.H2H_URL || "http://127.0.0.1:8000/graphql";
const QUERY = JSON.stringify({
  query:
    'query ProductList($first: Int) { products(first: $first, channel: "default-channel") { edges { node { id name slug productType { id name } } } } }',
  variables: { first: 20 },
});

export const options = { vus: 20, duration: "30s" };

export default function () {
  const res = http.post(URL, QUERY, { headers: { "Content-Type": "application/json" } });
  check(res, { ok: (r) => r.status === 200 && !r.json().errors });
}
