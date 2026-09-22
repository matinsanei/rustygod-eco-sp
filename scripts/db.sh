#!/usr/bin/env bash
# Backup / restore the Saleor PostgreSQL that rustygod-saleor shares with Django.
# Learned the hard way: database processes die, snapshots save weekends.
#
# The live DB is reached over TCP (default 127.0.0.1:5434 — override with
# PGHOST/PGPORT/POSTGRES_*); pg_* tools run from a throwaway postgres image
# on host networking, so no local postgres install is needed.
#
#   ./scripts/db.sh backup [name]       # custom-format dump -> backups/
#   ./scripts/db.sh restore <dump-file> # drop + recreate + pg_restore
#   ./scripts/db.sh psql [psql args]    # psql into the live DB
#   ./scripts/db.sh ready               # pg_isready against the live DB
#
# NOTE: the saleor-db container runs with --network host and postgres on
# **port 5434** (Cmd -p 5434), so bare `pg_isready`/`psql` *inside* the
# container fail on the default 5432. Always go through this script (or pass
# -p 5434 explicitly).
set -euo pipefail

PGHOST="${PGHOST:-127.0.0.1}"
PGPORT="${PGPORT:-5434}"
DB_USER="${POSTGRES_USER:-saleor}"
DB_NAME="${POSTGRES_DB:-saleor}"
export PGPASSWORD="${POSTGRES_PASSWORD:-saleor}"
PGIMG="${PG_IMAGE:-postgres:15-alpine}"

usage() {
  echo "usage: $0 backup [name] | $0 restore <dump-file>" >&2
  exit 1
}

run_pg() {
  docker run --rm --network host "$PGIMG" "$@"
}

case "${1:-}" in
  backup)
    name="${2:-saleor-$(date +%Y%m%d-%H%M%S).dump}"
    mkdir -p backups
    # --no-owner/--no-acl: restorable anywhere, no role drama.
    # (stdout redirect, NOT -f -: that yields empty output on pg15.)
    run_pg pg_dump -h "$PGHOST" -p "$PGPORT" -U "$DB_USER" -d "$DB_NAME" \
      -Fc --no-owner --no-acl > "backups/$name"
    echo "backup -> backups/$name ($(du -h "backups/$name" | cut -f1))"
    ;;
  restore)
    dump="${2:?usage: $0 restore <dump-file>}"
    test -f "$dump" || { echo "no such file: $dump" >&2; exit 1; }
    dir="$(cd "$(dirname "$dump")" && pwd)"
    base="$(basename "$dump")"
    run_pg psql -h "$PGHOST" -p "$PGPORT" -U "$DB_USER" -d postgres \
      -c "SELECT pg_terminate_backend(pid) FROM pg_stat_activity WHERE datname='$DB_NAME' AND pid <> pg_backend_pid();" \
      -c "DROP DATABASE IF EXISTS $DB_NAME;" \
      -c "CREATE DATABASE $DB_NAME OWNER $DB_USER;"
    docker run --rm --network host -v "$dir:/w:ro" "$PGIMG" \
      pg_restore -h "$PGHOST" -p "$PGPORT" -U "$DB_USER" -d "$DB_NAME" \
      --no-owner --no-acl "/w/$base"
    echo "restored $dump into $PGHOST:$PGPORT/$DB_NAME"
    ;;
  psql)
    shift
    run_pg psql -h "$PGHOST" -p "$PGPORT" -U "$DB_USER" -d "$DB_NAME" "$@"
    ;;
  ready)
    run_pg pg_isready -h "$PGHOST" -p "$PGPORT" -U "$DB_USER"
    ;;
  *)
    usage
    ;;
esac
