# tgselector

CLI-утилита, которая берёт JSON-экспорт из Telegram Desktop, находит сообщение по `id` и печатает готовый обработанный текст поста — в Markdown или в виде чистого текста.

В экспорте тело поста лежит в поле `text`, которое бывает либо простой строкой, либо массивом из строк и типизированных сущностей форматирования (`bold`, `code`, `pre`, `text_link` и т.д.). `tgselector` собирает из этого читаемый текст за один вызов.

## Установка

```sh
# через Homebrew (после первого релиза)
brew install jtprogru/tap/tgselector

# или из исходников
cargo build --release
# бинарь: ./target/release/tgselector
```

## Использование

```sh
# один пост
tgselector --file result.json --id 4

# несколько постов сразу (разделяются горизонтальной чертой)
tgselector --file result.json --id 4,5,19

# чистый текст вместо Markdown
tgselector --file result.json --id 4 --format plain

# с метаданными: дата, ссылка на пост, автор, репост, медиа
tgselector --file result.json --id 4 --meta

# JSON-структура для передачи агенту/скрипту
tgselector --file result.json --id 4,5 --json

# обзор последних 10 постов: id, дата, превью
tgselector --file result.json --list

# весь список целиком
tgselector --file result.json --list --list-full
```

| Флаг | Обязательный | Описание |
|------|--------------|----------|
| `-f`, `--file <PATH>` | да* | Путь к экспорту (`result.json`). |
| `-i`, `--id <IDS>` | да** | Id поста(ов) через запятую, напр. `4,5,19`. |
| `-l`, `--list` | да** | Список постов: `id <tab> дата <tab> превью` (по умолчанию последние 10). |
| `--list-full` | нет | В режиме `--list` показать весь список целиком. |
| `--list-limit <N>` | нет | Сколько последних постов показывать в `--list` (по умолчанию `10`). |
| `--list-width <N>` | нет | Ширина превью в `--list` (по умолчанию `60`). |
| `--format <markdown\|plain>` | нет | Формат вывода (по умолчанию `markdown`). |
| `--meta` | нет | Добавить футер с метаданными (дата, ссылка, автор, репост, медиа). |
| `--json` | нет | Вывести JSON-структуру вместо текста. |
| `--channel <USERNAME>` | нет | @username публичного канала для ссылок `t.me/<channel>/<id>`. |
| `--completions <SHELL>` | нет | Напечатать скрипт автодополнения (`bash`/`zsh`/`fish`/…). |

\* не нужен только для `--completions`. \*\* нужен либо `--id`, либо `--list`.

Результат печатается в stdout.

### JSON (`--json`)

Для передачи постов другому агенту/скрипту. Всегда возвращается массив объектов; поле `text` содержит текст в выбранном `--format`, метаданные лежат отдельными полями:

```json
[
  {
    "id": 4,
    "date": "2025-12-01T18:15:41",
    "link": "https://t.me/c/3373671933/4",
    "from": "Идеи для постов",
    "forwarded_from": "Kali Linux",
    "media": "photo",
    "format": "markdown",
    "text": "💀Безопасники опубликовали список…"
  }
]
```

В режиме `--list --json` возвращается массив `{id, date, preview}`.

### Метаданные (`--meta`)

После текста поста добавляется футер. Ссылка по умолчанию строится в формате `https://t.me/c/<chat_id>/<id>` (для приватных каналов открывается у участников). В экспорте Telegram нет публичного username, поэтому для публичного канала передайте его через `--channel <username>` — тогда ссылка будет `https://t.me/<username>/<id>`:

```
Текст поста…

---
📅 2025-12-01 18:15
🔗 https://t.me/c/3373671933/4
👤 Идеи для постов
♻️ переслано из: Kali Linux
📎 photo
```

Показываются только те строки, для которых есть данные.

### Коды возврата

| Код | Значение |
|-----|----------|
| `0` | Успех (текст выведен). |
| `1` | Файл не читается или JSON невалиден. |
| `2` | Сообщение с указанным `id` не найдено. |

## Поддерживаемые сущности

| Тип | Markdown | Plain |
|-----|----------|-------|
| `plain` / голая строка | как есть | как есть |
| `bold` | `**text**` | `text` |
| `italic` | `*text*` | `text` |
| `underline` | `<u>text</u>` | `text` |
| `strikethrough` | `~~text~~` | `text` |
| `code` | `` `text` `` | `text` |
| `pre` | блок ```` ``` ```` с языком | `text` |
| `blockquote` | каждая строка с `> ` | `text` |
| `spoiler` | `\|\|text\|\|` | `text` |
| `link` | `text` (URL) | `text` |
| `text_link` | `[text](href)` | `text (href)` |
| `mention`, `hashtag`, `phone`, `bot_command`, `custom_emoji` | `text` | `text` |
| неизвестный тип | `text` (fallback) | `text` |

## Автодополнение

`tgselector --completions <shell>` печатает скрипт автодополнения в stdout. Через Homebrew он ставится автоматически. Вручную:

```sh
# zsh (положить в каталог из $fpath, напр.)
tgselector --completions zsh > ~/.zfunc/_tgselector

# bash
tgselector --completions bash > /usr/local/etc/bash_completion.d/tgselector

# fish
tgselector --completions fish > ~/.config/fish/completions/tgselector.fish
```

Поддерживаются `bash`, `zsh`, `fish`, `elvish`, `powershell`.

## Разработка

```sh
make build   # сборка
make test    # тесты
make lint    # clippy с -D warnings
make fmt     # форматирование
```
