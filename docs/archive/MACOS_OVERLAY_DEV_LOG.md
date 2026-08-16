# macOS overlay window — лог разработки

Хронология всех шагов, экспериментов и решений по пути к текущей
реализации оверлей-окна питомца. Итоговое поведение и объяснение "как
это работает сейчас" — в
[`docs/mechanics/MACOS_OVERLAY_WINDOW.md`](../mechanics/MACOS_OVERLAY_WINDOW.md).
Этот файл — история, а не справочник.

## 1. Первая постановка задачи

Запрос: окно поверх всех окон, на всех Spaces, прозрачное, мышь должна
работать только по спрайту, а остальная часть окна — прокликиваться
насквозь (click-through). Идею с click-through по спрайту отложили как
слишком рискованную для первой итерации — упростили до просто
плавающего окна без прокликивания.

## 2. Базовая настройка окна

`tauri.conf.json`: `transparent: true`, `decorations: false`,
`alwaysOnTop: true`, `shadow: false`. Прозрачность на macOS не
заработала без `macOSPrivateApi: true` в `app`-секции — добавили.

## 3. Перетаскивание окна

`data-tauri-drag-region` в `App.vue` молча не работал — недоставало
ACL-разрешения `core:window:allow-start-dragging` (без ошибки в
консоли, просто ничего не происходило). После добавления разрешения
драг заработал только с второго клика — курсор сначала пытался
выделить текст (I-beam). Причина: `data-tauri-drag-region` реагирует
только на элемент, на котором стоит атрибут, а не на родителей —
добавили атрибут и на сам спрайт, плюс `user-select: none` глобально.

## 4. Автоподгонка размера окна под спрайт

`getCurrentWindow().setSize(...)` из фронтенда требовал ещё одно ACL
разрешение — `core:window:allow-set-size`.

## 5. Первая попытка не воровать фокус

Клик по окну переключал активное приложение. `window.set_focusable(false)`
не помог — оказалось, что "может ли окно стать key window" и "активирует
ли клик всё приложение целиком" — разные, независимые механизмы AppKit.

Следующая попытка — `ActivationPolicy::Accessory` (агентское приложение,
без Dock-иконки) — фокус перестал воровать, но заодно приложение исчезло
из Dock и Mission Control/Cmd+Tab. Рабочий, но с побочным эффектом.

## 6. Первая попытка попасть в fullscreen-Space

`collectionBehavior |= FullScreenAuxiliary` на обычном `NSWindow` не
дал видимости над fullscreen-приложением — окно формально было в
Space, но WindowServer рисовал его позади контента.

## 7. Ключевая находка через Swift-прототип

Пользователь параллельно собирал прототип питомца на SwiftUI и принёс
результаты исследования: `fullScreenAuxiliary` реально работает только
у окон с `.titled` в `styleMask` — полностью `.borderless` окна
WindowServer исключает из compositing над fullscreen-Space. Также
нашли, что видимую "таблетку" тайтлбара после `titlebarAppearsTransparent`
пришлось скрывать вручную — поиском `NSTitlebarContainerView` по имени
класса в дереве `contentView().superview()`.

Перенесли находку в проект: `decorations: true` (даёт `.titled`),
`styleMask = Titled | FullSizeContentView`, `titlebarAppearsTransparent`
+ `titleVisibility: Hidden`, приватный поиск `NSTitlebarContainerView`.
Fullscreen заработал. Mission Control по-прежнему был спрятан
(наследие `ActivationPolicy::Accessory` из шага 5).

## 8. Референс-проект: NSPanel вместо Accessory

Пользователь нашёл в соседнем проекте (`petdex`, native-sdk на Zig)
готовый рабочий рецепт для точно такой же desktop-companion задачи:
`NSPanel` (не `NSWindow`) со `styleMask` включающим `NonactivatingPanel`,
плюс `floatingPanel`, `becomesKeyOnlyIfNeeded`, `hidesOnDeactivate`,
`collectionBehavior` с `Stationary` и `CanJoinAllApplications`,
`level = NSScreenSaverWindowLevel`. Активность приложения при этом
остаётся `Regular` — не нужен `Accessory` вообще.

Реализовали через isa-swizzle: `object_setClass(ns_window_ptr, NSPanel::class())`
в Rust через `objc2`/`objc2-app-kit` (у `tao`/Tauri нет способа создать
окно сразу как `NSPanel` — окно уже существует к моменту, когда
`setup()` получает к нему доступ). Убрали `ActivationPolicy::Accessory`.

Результат: одновременно фокус не воруется, видно над fullscreen, и
Dock/Mission Control не пропадают. Основная задача решена.

## 9. Ложная тревога с "исчезновением" на fullscreen

Питомец иногда пропадал и телепортировался на обычный Space при клике
внутри fullscreen-приложения. Разбор консоли `tauri dev` показал:
`File src-tauri/Cargo.toml changed. Rebuilding application...` — это
были собственные правки пользователя в `Cargo.toml` (добавление
комментариев), из-за которых файловый вотчер `tauri dev` полностью
перезапускал процесс. Не баг оконной логики — в проде (`tauri build`)
такого вотчера нет вообще.

## 10. `acceptFirstMouse`

Отдельно от воровства фокуса — первый клик по неактивному окну не
доходил до вебвью вообще (стандартное поведение macOS: первый клик
"будит" окно, второй — действует). Добавили `acceptFirstMouse: true` в
конфиг — драг заработал с первого клика.

## 11. Проверка фактов в документации

При написании доки часть формулировок оказались непроверенными
предположениями:
- `NSWindowCollectionBehaviorStationary` — проверили точный текст
  заголовков Apple: *"unaffected by exposé"* — про Exposé, а не про
  анимацию свайпа между Spaces, как было написано изначально.
- `setReleasedWhenClosed(false)` — идея была взята из бага в **другом**
  проекте (свой AppKit-хост с нуля), не проверена на нашем коде, где
  окно вообще не закрывается индивидуально. Закомментировали до тех
  пор, пока не появится реальный сценарий закрытия.

## 12. Эмпирическая проверка: действительно ли нужен именно `NSPanel`

По просьбе — отключили `object_setClass`, оставив всё остальное как
есть (обычный `NSWindow` с тем же `styleMask`/`collectionBehavior`).
Подтвердилось: fullscreen-видимость и отсутствие кражи фокуса
одновременно пропали. `object_setClass` — не эстетика, а необходимость.

## 13. Поиск публичной альтернативы приватному `NSTitlebarContainerView`

Три эксперимента подряд:
1. Toggle `styleMask` в `empty()` и обратно, чтобы форсировать AppKit
   пересобрать хром окна — **краш**.
2. `decorations: false` + вообще не вызывать `setStyleMask` (честно
   `.borderless` `NSPanel`) — не крашится, но fullscreen-видимость **не
   работает**: `.titled` обязателен даже для `NSPanel`, не только для
   `NSWindow`.
3. `decorations: false` + добавление `.titled` в коде постфактум —
   **краш**.

Вывод на тот момент: приватный поиск `NSTitlebarContainerView` — 
единственный работающий вариант, `.titled` должен быть выставлен с
момента создания окна и не менять значение.

## 14. Разбор настоящей причины краша

Вместо дальнейших догадок — посмотрели реальные краш-репорты macOS
(`~/Library/Logs/DiagnosticReports/petumec-*.ips`) от экспериментов
шага 13. Краш оказался **не в нашем коде**, а внутри `tao`:

```
NSApplication _sendFinishLaunchingNotification
  → tao::platform_impl::platform::app_delegate::did_finish_launching
    → objc2 message_receiver::send_message
      → panic_cannot_unwind → abort
```

`setup()` Tauri выполняется как часть `did_finish_launching`. Наш код
менял класс/`styleMask` окна синхронно, прямо там же — `tao`,
продолжая свою собственную логику следом за нами в том же коллбэке,
натыкался на уже изменённое состояние и падал.

## 15. Фикс: `dispatch_async` — деференс на следующий тик run loop

Тот же приём, что использовался в самом первом Swift-прототипе
(`DispatchQueue.main.async { configure(view) }`). Реализовали через
`dispatch_async_f` на главную очередь (`_dispatch_main_q`) — вся
настройка окна теперь выполняется не сразу в `setup()`, а на следующем
тике, уже после того как `tao` полностью закончил `did_finish_launching`.

Краши пропали. Заодно — не запрашивая этого специально — пропала и
"таблетка" тайтлбара: `titlebarAppearsTransparent` + `titleVisibility:
Hidden`, применённые уже после гонки с `tao`, полностью скрывают бар
безо всякого приватного поиска. Убрали код поиска
`NSTitlebarContainerView` — в решении больше не осталось приватного
API AppKit вообще, только публичный `object_setClass`.

## 16. Минимизация native-кода — перенос дублей в конфиг

Прошлись по каждой строке `configure()`, сверяя с
[справочником Tauri](https://v2.tauri.app/reference/config/#windowconfig):
что из этого уже есть как поле `tauri.conf.json`.

Убрали как чистые дубли (уже стояли в конфиге, эффекта не меняли):
`setOpaque(false)` (покрыто `transparent: true`), `setHasShadow(false)`
(покрыто `shadow: false`), `collectionBehavior |= CanJoinAllSpaces`
(покрыто `visibleOnAllWorkspaces: true`).

Перенесли и протестировали: `titleBarStyle: "Overlay"` +
`hiddenTitle: true` в конфиге заменили ручные
`titlebarAppearsTransparent`/`titleVisibility`/`FullSizeContentView` —
`styleMask` в коде теперь читается и дополняется одним битом
(`NonactivatingPanel`), а не задаётся заново целиком.

Осталось в Rust — то, чему в Tauri нет декларативного аналога вообще:
`object_setClass`, `NonactivatingPanel`, `floatingPanel`,
`becomesKeyOnlyIfNeeded`, `hidesOnDeactivate`, `FullScreenAuxiliary`,
`Stationary`, `CanJoinAllApplications`, `level`.

## 17. Проверка: действительно ли нужен `level = 1000`

Единственная настройка, которая была в коде с самого начала и ни разу
не тестировалась в изоляции. Убрали `setLevel(1000)`, оставив только
`alwaysOnTop: true` из конфига (даёт `NSFloatingWindowLevel`) —
fullscreen-видимость пропала. Вернули `setLevel(1000)`. Подтверждено
эмпирически, не по референсу на слово.

## Итог

От первой рабочей версии до текущей прошло несколько полных циклов
"гипотеза → тест → факт", включая минимум один раз, когда собственная
формулировка в доке (`Stationary`) оказалась неверной при проверке.
Финальное решение короче, стабильнее и не использует приватный API
AppKit — не потому что так спланировали заранее, а потому что случайно
обнаруженный фикс одной проблемы (гонка с `tao`) заодно устранил и
другую (приватный поиск тайтлбара).
