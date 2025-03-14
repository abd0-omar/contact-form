#!/bin/bash

TARGET_DIR="scripts/a_place_for_test_dbs_to_spawn_in_it,supposed_to_be_empty_cuz_tests_terminate_after_success_execution"

# Ensure the directory exists
if [[ ! -d "$TARGET_DIR" ]]; then
  echo "❌ Error: Directory '$TARGET_DIR' does not exist!"
  exit 1
fi

# Move into the directory
cd "$TARGET_DIR" || { echo "❌ Error: Failed to enter '$TARGET_DIR'"; exit 1; }

# Get the script's filename
script_name=$(basename "$0")

for file in *; do
  if [[ "$file" != "$script_name" ]]; then
    rm -rf -- "$file"
  fi
done

echo "✅ Cleanup complete."
