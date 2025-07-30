#!/bin/bash
set -e

mkdir -p migrations
rm -f migrations/*.sql

tables=(test_table_1 test_table_2 test_table_3 test_table_4 test_table_5)
ops=(CREATE INSERT UPDATE DELETE DROP)
version=1

for tbl in "${tables[@]}"; do
  for op in "${ops[@]}"; do
    padded_version=$(printf "%04d" "$version")
    fname="migrations/${padded_version}_${op,,}_${tbl}.sql"

    case $op in
    CREATE)
      echo "CREATE TABLE IF NOT EXISTS $tbl (id INT PRIMARY KEY, data TEXT);" >"$fname"
      ;;
    INSERT)
      echo "INSERT INTO $tbl (id, data) VALUES ($RANDOM, 'test_data_$RANDOM');" >"$fname"
      ;;
    UPDATE)
      echo "UPDATE $tbl SET data = 'updated_$RANDOM' WHERE id = $((RANDOM % 100));" >"$fname"
      ;;
    DELETE)
      echo "DELETE FROM $tbl WHERE id = $((RANDOM % 100));" >"$fname"
      ;;
    DROP)
      echo "DROP TABLE IF EXISTS $tbl;" >"$fname"
      ;;
    esac

    ((version++))
  done
done
