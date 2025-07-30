import os
import re
import glob
import subprocess
import sqlite3
import pyodbc

DB_FILE = "test.db"
DSN_NAME = "test_sqlite_test"
DUMP_FILE = "schema_dump.sql"
MIGRATIONS_DIR = "migrations"


def step(msg):
    print(f"[*] {msg}")


def fail(msg):
    print(f"[❌] {msg}")
    exit(1)


def ok():
    print("[✅] All assertions passed")


def reset_db():
    step(f"Resetting database: {DB_FILE}")
    if os.path.exists(DB_FILE):
        os.remove(DB_FILE)
    with sqlite3.connect(DB_FILE) as conn:
        conn.execute("VACUUM;")


def verify_dsn():
    step(f"Verifying DSN exists: {DSN_NAME}")
    odbc_ini = os.path.expanduser("~/.odbc.ini")
    with open(odbc_ini) as f:
        content = f.read()
        if not re.search(rf"^\[{re.escape(DSN_NAME)}\]", content, re.MULTILINE):
            fail(f"DSN '{DSN_NAME}' not found in {odbc_ini}")


def count_migrations():
    files = sorted(glob.glob(f"{MIGRATIONS_DIR}/*.sql"))
    step(f"Found {len(files)} migration files")
    return len(files)


def run_migrations():
    step("Applying migrations")
    subprocess.run(
        ["cargo", "run", "--quiet", "--", "apply", "--conn", f"DSN={DSN_NAME};"],
        check=True,
    )


def verify_migration_status(expected_count):
    step("Verifying migration status")
    result = subprocess.check_output(
        ["cargo", "run", "--quiet", "--", "status", "--conn", f"DSN={DSN_NAME};"]
    ).decode()
    print(result)

    match = re.search(r"Applied:\s+(\d+)", result)
    if not match:
        fail("Could not parse applied migration count")
    applied = int(match.group(1))
    if applied != expected_count:
        fail(f"Expected {expected_count} applied migrations, got {applied}")


def dump_schema():
    step(f"Dumping schema to: {DUMP_FILE}")
    conn = pyodbc.connect(f"DSN={DSN_NAME}")
    with open(DUMP_FILE, "w") as f:
        for query in [
            "SELECT sql FROM sqlite_master WHERE type='table' AND name NOT LIKE 'sqlite_%';",
            "SELECT sql FROM sqlite_master WHERE type='index' AND sql NOT NULL;",
        ]:
            cur = conn.cursor()
            for row in cur.execute(query):
                if row[0]:
                    f.write(row[0] + "\n")


def extract_objects(keyword):
    """
    Returns a set of object names (lowercased) for a given SQL keyword.
    e.g., CREATE TABLE, DROP TABLE, CREATE INDEX
    """
    pattern = rf"{keyword}\s+(IF\s+(NOT\s+)?EXISTS\s+)?[`\"[]?(\w+)[`\"\]]?"
    result = set()
    for file in glob.glob(f"{MIGRATIONS_DIR}/*.sql"):
        with open(file) as f:
            content = f.read()
            matches = re.findall(pattern, content, flags=re.IGNORECASE)
            for _, _, name in matches:
                result.add(name.lower())
    return result


def get_schema_lines():
    with open(DUMP_FILE) as f:
        return f.read().splitlines()


def check_table_exists(name, schema_lines):
    pattern = rf"CREATE TABLE.*[`\"[]?{re.escape(name)}[`\"\]]?"
    return any(re.search(pattern, line, re.IGNORECASE) for line in schema_lines)


def check_index_exists(name, schema_lines):
    return any(name.lower() in line.lower() for line in schema_lines)


def validate_tables(schema_lines):
    step("Checking table definitions")
    created = extract_objects("CREATE TABLE")
    dropped = extract_objects("DROP TABLE")
    to_check = sorted(created - dropped)
    for tbl in to_check:
        if not check_table_exists(tbl, schema_lines):
            fail(f"Missing table: {tbl}")
    for tbl in sorted(created & dropped):
        print(f"[*] Skipped dropped table: {tbl}")


def validate_indexes(schema_lines):
    step("Checking index definitions")
    indexes = extract_objects("CREATE INDEX")
    for idx in sorted(indexes):
        if not check_index_exists(idx, schema_lines):
            fail(f"Missing index: {idx}")


def main():
    reset_db()
    verify_dsn()
    expected_migrations = count_migrations()
    run_migrations()
    verify_migration_status(expected_migrations)
    dump_schema()
    schema_lines = get_schema_lines()
    validate_tables(schema_lines)
    validate_indexes(schema_lines)
    ok()


if __name__ == "__main__":
    main()
