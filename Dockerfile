# --- ЭТАП 1: Сборка (Builder) ---
FROM rust:1.92-slim-trixie as builder

# Устанавливаем системные зависимости для сборки
# pkg-config и libfuse-dev нужны для работы крейта fuser
RUN apt-get update && apt-get install -y pkg-config libfuse3-dev

WORKDIR /usr/src/app

# ХАК ДЛЯ КЭША:
# 1. Создаем пустой проект
RUN cargo new --bin mc-anvil-db
WORKDIR /usr/src/app/mc-anvil-db

# 2. Копируем только списки зависимостей
COPY ./Cargo.toml ./Cargo.lock ./

# 3. Билдим "пустышку". Это скачает и скомпилирует все библиотеки (redis, fuser и т.д.)
# Docker закэширует этот слой.
RUN cargo build --release

# 4. Удаляем исходники пустышки
RUN rm src/*.rs

# 5. Теперь копируем ТВОЙ реальный код
COPY ./src ./src

# 6. Билдим реальный проект.
# Docker увидит, что зависимости не изменились, и пересобирать их не будет!
# Будет пересобираться только твой main.rs (это секунды).
# touch нужен, чтобы обновить время файла и заставить cargo пересобрать его
RUN rm ./target/release/deps/mc_anvil_db*
RUN cargo build --release

# --- ЭТАП 2: Запуск (Runtime) ---
FROM debian:trixie-slim

# Устанавливаем fuse3 и tini (для правильной обработки сигналов)
RUN apt-get update && apt-get install -y fuse3 tini ca-certificates && rm -rf /var/lib/apt/lists/*

# Копируем бинарник из первого этапа
COPY --from=builder /usr/src/app/mc-anvil-db/target/release/mc-anvil-db /usr/local/bin/mc-anvil-db

# Копируем entrypoint скрипт
COPY entrypoint.sh /usr/local/bin/entrypoint.sh
RUN chmod +x /usr/local/bin/entrypoint.sh

# Настраиваем окружение
ENV RUST_LOG=info

# Используем tini как init и наш entrypoint для graceful shutdown
ENTRYPOINT ["/usr/bin/tini", "--", "/usr/local/bin/entrypoint.sh"]