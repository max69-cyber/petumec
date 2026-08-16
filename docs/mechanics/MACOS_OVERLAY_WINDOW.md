# macOS overlay window

История экспериментов для получения конечного результата:
[`docs/archive/MACOS_OVERLAY_DEV_LOG.md`](../archive/MACOS_OVERLAY_DEV_LOG.md).

## Как должно себя вести окно

- прозрачное, без видимой рамки/тайтлбара
- всегда поверх остальных окон, видно на всех Spaces
- видно **поверх fullscreen-приложений** на соседнем Space
- клик/драг по нему **не переключает** активное приложение (не ворует
  фокус)
- перетаскивается за фон

## Через `tauri.conf.json`

| Поле | Зачем |
|---|---|
| `decorations: true` | база — без неё `tao` не создаст `.titled`-окно, а без `.titled` не работает fullscreen-compositing |
| `titleBarStyle: "Overlay"` | контент растягивается под тайтлбар, бар прозрачный |
| `hiddenTitle: true` | скрывает текст заголовка |
| `transparent: true` | прозрачный фон окна (нужен `macOSPrivateApi: true`) |
| `shadow: false` | без системной тени окна |
| `alwaysOnTop: true` | базовый floating-уровень |
| `visibleOnAllWorkspaces: true` | видно на всех обычных Spaces |
| `acceptFirstMouse: true` | первый клик сразу доходит до вебвью, не "съедается" системой |
| `focus: false` | не забирает фокус при старте |
| `resizable/closable/minimizable/maximizable: false` | никаких системных кнопок/ресайза |

## Через `src-tauri/src/lib.rs` (нативный код)

То, чему нет аналога в конфиге Tauri — все свойства `NSPanel` и низкоуровневые флаги AppKit:

| Свойство | Зачем |
|---|---|
| `object_setClass(...)` → `NSPanel` | ключевая вещь — только у объектов класса `NSPanel` реально работают `NonactivatingPanel` и compositing над fullscreen. У Tauri нет способа создать окно сразу как `NSPanel` |
| `styleMask \|= NonactivatingPanel` | не ворует фокус приложения при клике |
| `floatingPanel = true` | ведёт себя как плавающая вспомогательная панель |
| `becomesKeyOnlyIfNeeded = true` | не становится key window на обычный клик |
| `hidesOnDeactivate = false` | не прячется вместе с остальными окнами приложения |
| `collectionBehavior \|= FullScreenAuxiliary` | участвует в compositing над fullscreen-Space |
| `collectionBehavior \|= Stationary` | не участвует в раскладке при Exposé |
| `collectionBehavior \|= CanJoinAllApplications` | можно быть поверх *чужого* fullscreen-приложения (macOS 13+) |
| `level = 1000` (`NSScreenSaverWindowLevel`) | выше, чем даёт `alwaysOnTop` сам по себе — иначе не видно над fullscreen |

## Как это применяется через Rust/`tao`

Tauri создаёт окно через `tao` уже с настройками из конфига выше. В
`setup()` Tauri-приложения мы получаем указатель на нативный `NSWindow`
через `window.with_webview(...)` — но **не трогаем его сразу**.

Вся настройка обёрнута в `extern "C" fn configure(...)` и запускается
через `dispatch_async_f` на главную очередь — то есть выполнится только на
**следующем тике run loop**, уже после того как `tao` полностью
закончит свою часть инициализации окна (`did_finish_launching`).
Причина: применение этих же настроек синхронно, сразу в `setup()`,
конфликтовало с внутренним кодом `tao` и крашило приложение —
подробности и разбор краш-репорта в полной версии доки.
