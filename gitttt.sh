#!/usr/bin/env bash

set -e

echo "Ищем вложенные .git директории..."

# Текущая директория
ROOT_DIR="$(pwd)"

# Найти все .git директории, кроме корневой
find "$ROOT_DIR" -type d -name ".git" ! -path "$ROOT_DIR/.git" | while read -r gitdir; do
    echo "Удаляем: $gitdir"
    rm -rf "$gitdir"
done

echo "Готово."