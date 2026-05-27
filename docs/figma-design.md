# ConfigPilot UI Design Spec for Figma AI

## Design System: shadcn/ui + React

Use the **shadcn/ui** component library as the design system foundation. All components reference standard shadcn/ui naming and patterns. The app is a **macOS desktop application** (Tauri), so design with a compact, native-feel desktop layout — not a mobile-responsive web page.

---

## 1. Global Layout

### App Shell

```
+-------------------------------------------------------+
|  [Sidebar]  |         [Main Content Area]              |
|             |                                          |
|  Logo       |   Page Header                            |
|  Nav items  |   (title + action buttons)               |
|             |   ----------------------------           |
|  - Home     |                                          |
|  - Sync     |   Page Content                           |
|  - Conflicts|                                          |
|  - Settings |                                          |
|             |                                          |
|  -----------|                                          |
|  Status bar |   Status / Toast messages                |
+-------------------------------------------------------+
```

- **Window default size**: 960×640px, min 800×540px
- **Sidebar**: fixed 220px width, bg=slate-900, text=slate-100
- **Main content**: flex-1, bg=white (light mode) / slate-950 (dark mode), padding=24px
- **Font**: Inter (system default), size=14px base
- **Border radius**: 6px (cards), 8px (buttons), 4px (inputs)
- **Spacing unit**: 4px base (p-2=8px, p-4=16px, p-6=24px)

---

## 2. Color Tokens (shadcn compatible)

### Light Mode

| Token | Hex |
|---|---|
| background | #FFFFFF |
| foreground | #0F172A |
| card | #FFFFFF |
| card-foreground | #0F172A |
| primary | #18181B |
| primary-foreground | #FAFAFA |
| secondary | #F4F4F5 |
| secondary-foreground | #18181B |
| muted | #F1F5F9 |
| muted-foreground | #64748B |
| accent | #F4F4F5 |
| accent-foreground | #18181B |
| destructive | #EF4444 |
| destructive-foreground | #FAFAFA |
| border | #E2E8F0 |
| ring | #94A3B8 |
| success | #22C55E |
| warning | #F59E0B |

### Dark Mode

| Token | Hex |
|---|---|
| background | #0F172A |
| foreground | #F8FAFC |
| card | #1E293B |
| card-foreground | #F8FAFC |
| primary | #FAFAFA |
| primary-foreground | #18181B |
| secondary | #334155 |
| secondary-foreground | #F8FAFC |
| muted | #1E293B |
| muted-foreground | #94A3B8 |
| border | #334155 |
| destructive | #7F1D1D |
| ring | #475569 |

---

## 3. Sidebar Component

```
+---------------------+
|                     |
|  [App Icon]         |  32×32 app icon, bg=#18181B, rounded=8px
|  ConfigPilot        |  text-lg, font-semibold, text=slate-100
|                     |
|  ─────────────────  |  Divider, border=slate-700
|                     |
|  [icon] Home        |  Nav item: icon (lucide) + label
|  [icon] Sync        |  Active: bg=slate-700, left-border=3px primary
|  [icon] Conflicts   |  Hover: bg=slate-800
|  [icon] Settings    |  Inactive: transparent
|                     |  Icon size=18px, gap=12px, padding-y=8px, padding-x=16px
|                     |
|                     |
|  ─────────────────  |
|  Status indicator   |  sync_status: "idle"=gray dot, "syncing"=blue spinner, "error"=red dot
|  v0.1.0             |  muted-foreground, text-xs
+---------------------+
```

**Nav items with icons:**
- Home → `LayoutDashboard` (lucide)
- Sync → `RefreshCw`
- Conflicts → `GitFork`
- Settings → `Settings`

**Conflict badge**: When conflicts > 0, show a red badge (Badge component, variant="destructive") next to "Conflicts" with the count number.

---

## 4. Page 1: Home (Dashboard)

### Layout

```
+----------------------------------------------------------+
|  首页总览                          [开启监听] [刷新]       |
|                                                           |
|  +------------------+  +------------------+  +--------------+
|  | GitHub 授权       |  | 仓库             |  | 已发现配置    |   ← Card components
|  | ✅ 已授权         |  | ranen/config... |  | 4 / 6        |
|  | username: ranen   |  | 🔒 私有仓库      |  |              |
|  +------------------+  +------------------+  +--------------+
|
|  +------------------+  +------------------+
|  | 最近同步          |  | 自动监听         |
|  | 2 分钟前          |  | 🟢 运行中        |
|  | commit: a1b2c3d  |  | 已监听 4 个文件   |
|  +------------------+  +------------------+
|
|  ───────────────────────────────────────────────────────
|
|  配置文件清单
|
|  +------------------------------------------------------+
|  | 配置名称      | 源路径              | 仓库路径    | 状态 |
|  |---------------+--------------------+------------+------|
|  | .zshrc       | ~/.zshrc           | zsh/.zshrc | ✅   |   ← Table component
|  | .zprofile    | ~/.zprofile        | ...        | ✅   |
|  | config/ghostty| ~/.config/ghostty  | ...        | ✅   |
|  +------------------------------------------------------+
+----------------------------------------------------------+
```

### Components Used

**Stats Cards Row (5 cards, 2 rows):**
- Use `Card` + `CardHeader` + `CardContent`
- Each card: icon (16px, muted-foreground) + label (text-sm, muted-foreground) + value (text-xl, font-bold)
- Card width: flex, min-w-0, equal distribution
- Examples:
  - **GitHub Auth**: icon=`Github` (lucide), label="GitHub 授权", value=username or "未授权", status dot color
  - **Repo**: icon=`FolderGit2`, label="仓库", value="owner/repo", badge="私有"
  - **Configs Found**: icon=`FileText`, label="已发现配置", value="4 / 6"
  - **Last Sync**: icon=`Clock`, label="最近同步", value="2 分钟前" or "尚未同步", sub=commit hash
  - **Auto Watch**: icon=`Radio`, label="自动监听", value="运行中"/"已停止", status dot

**Action Buttons (top right):**
- `Button` variant="outline" for "刷新"
- `Button` for "开启监听" / "停止监听" (toggle style)

**Config Files Table:**
- Use `Table` + `TableHeader` + `TableBody` + `TableRow` + `TableCell`
- Columns: 配置名称 | 源路径 | 仓库路径 | 状态
- Path cells: `Tooltip` wrapping truncated text (max-w-[200px], truncate)
- Status: `Badge` variant="success" (green) or variant="secondary" (gray "未发现")
- Empty state: Table with single row "未发现配置文件" via `TableCell` colspan=4, text-center, muted-foreground

**Status colors:**
- Connected/synced → green dot (#22C55E)
- Disconnected → gray dot (#94A3B8)
- Warning → amber dot (#F59E0B)
- Error → red dot (#EF4444)

---

## 5. Page 2: GitHub Authorization

### Layout

```
+----------------------------------------------------------+
|  授权                                          ← Back     |
|                                                           |
|  +------------------------------------------------------+
|  |                                                      |
|  |        [GitHub Logo - 64px]                          |
|  |                                                      |
|  |        连接 GitHub 以开始同步                          |
|  |        将创建私有仓库 configpilot-dotfiles              |
|  |                                                      |
|  |        [  GitHub 登录  ]  ← Button size="lg"         |
|  |        验证码备用登录    ← Button variant="link"       |
|  |                                                      |
|  +------------------------------------------------------+
|
|  ── OR ── (when device flow active) ──
|
|  +------------------------------------------------------+
|  |                                                      |
|  |        验证码登录                                     |
|  |                                                      |
|  |        [ ABCD-1234 ]  ← large code display           |
|  |                                                      |
|  |        1. 打开 github.com/login/device               |
|  |        2. 输入上方验证码                               |
|  |        3. 点击授权                                    |
|  |                                                      |
|  |        [ 取消 ] [ 我已授权 ]                          |
|  +------------------------------------------------------+
+----------------------------------------------------------+
```

### Components Used

- **OAuth card**: `Card` centered, max-w-[420px], mx-auto, mt-12
- **GitHub icon**: lucide `Github` icon, size=64, mx-auto
- **Title**: text-xl, font-semibold
- **Description**: text-sm, muted-foreground
- **Primary button**: `Button` size="lg", variant="default", full width
- **Secondary link**: `Button` variant="link", size="sm"
- **Device flow code**: `div` with monospace font, text-2xl, tracking-widest, letter-spacing, border, rounded, px-6, py-3, text-center, select-all, bg=secondary
- **Steps**: ordered list with text-sm, muted-foreground, gap-2
- **Action buttons**: Row of `Button` variant="outline" + `Button`
- **Error state**: `Alert` variant="destructive" with error message

**States:**
1. **Initial**: Show OAuth card with "GitHub 登录" button
2. **Loading**: Button shows `Loader2` spinner icon + "等待授权..."
3. **Device Flow**: Toggle to device flow card with verification code
4. **Authorized**: `Badge` variant="success" + username displayed, button disabled
5. **Error**: `Alert` variant="destructive" below the card

---

## 6. Page 3: Sync

### Layout

```
+----------------------------------------------------------+
|  同步                                    [刷新状态]       |
|                                                           |
|  +------------------------------------------------------+
|  |  同步操作                                             |
|  |                                                      |
|  |  [上传] 立即备份      将本地配置推送到 GitHub 仓库     |
|  |  [下载] 从云端恢复    从 GitHub 仓库恢复到本地         |
|  |  [交叉] 双向同步      比较并同步本地与远端差异          |
|  +------------------------------------------------------+
|
|  ───────────────────────────────────────────────────────
|
|  同步状态
|
|  +------------------+  +------------------+  +--------------+
|  | 最近同步          |  | 最近 Commit       |  | 同步状态      |
|  | 2024-01-15 14:30  |  | a1b2c3d         |  | ✅ 已完成     |
|  +------------------+  +------------------+  +--------------+
|
|  ───────────────────────────────────────────────────────
|
|  配置文件
|
|  +------------------------------------------------------+
|  | [复选框] 配置名称      | 源路径         | 状态 | 操作  |
|  |------------------------------------------------------|
|  | [✓]     .zshrc       | ~/.zshrc      | ✅  | ...   |
|  | [✓]     .zprofile    | ~/.zprofile   | ✅  | ...   |
|  +------------------------------------------------------+
|
|  ───────────────────────────────────────────────────────
|
|  同步日志
|
|  +------------------------------------------------------+
|  | 14:30:21  备份完成 (4 个文件)                         |
|  | 14:29:01  连接到 GitHub 仓库                          |
|  | 14:28:55  开始备份...                                 |
|  +------------------------------------------------------+
+----------------------------------------------------------+
```

### Components Used

**Sync Actions Card:**
- `Card` with 3 action rows
- Each row: `Button` variant="outline" with icon + title + description
- Icons: `Upload` (backup), `Download` (restore), `ArrowLeftRight` (bidirectional)
- Button layout: icon left, text stacked (title bold, desc muted-foreground)
- Disabled state when not authorized: opacity-50, cursor-not-allowed
- Loading state: button icon replaced with `Loader2` spinner, disabled

**Sync Status Cards:**
- 3 `Card` components in a row (same pattern as Home page stats)
- Last sync time, last commit hash, sync status badge

**Config Files Table:**
- Same `Table` component as Home page
- Adds checkbox column (Checkbox component) for selective sync
- Adds "操作" (actions) column with small icon buttons

**Sync Log:**
- `Card` with `ScrollArea` inside
- Each log line: timestamp (text-xs, muted-foreground) + message (text-sm)
- Latest entry at top
- Max height: 200px, overflow-y: auto
- Empty state: "暂无同步日志" centered, muted-foreground

**Progress State:**
- When sync is in progress, show a `Progress` bar at the top of the page
- `Skeleton` loading for the status cards while fetching

---

## 7. Page 4: Conflicts

### Layout

```
+----------------------------------------------------------+
|  冲突处理                                 3 个冲突        |
|                                                           |
|  +------------------------------------------------------+
|  |                                                      |
|  |  检测到本地与远端配置存在差异，请选择保留哪个版本        |
|  |                                                      |
|  +------------------------------------------------------+
|
|  +------------------------------------------------------+
|  | 冲突 #1                                              |
|  |                                                      |
|  |  源路径:     ~/.zshrc                                 |
|  |  仓库路径:    zsh/.zshrc                              |
|  |                                                      |
|  |  +--------------------+  +-------------------------+  |
|  |  | 本地版本            |  | 远端版本                 |  |
|  |  | hash: abc123...     |  | hash: def456...         |  |
|  |  | 修改时间: ...       |  | 修改时间: ...           |  |
|  |  | [查看文件]          |  | [查看文件]               |  |
|  |  +--------------------+  +-------------------------+  |
|  |                                                      |
|  |  [使用本地]  [使用远端]  [打开源文件]                  |
|  +------------------------------------------------------+
|
|  ... (repeat for each conflict)
+----------------------------------------------------------+
```

### Components Used

**Page Header:**
- Title + `Badge` showing conflict count (variant="destructive" if > 0)

**Info Alert:**
- `Alert` variant="default" with info icon explaining the situation

**Conflict Cards:**
- Each conflict is a `Card` with:
  - Title: "冲突 #N" (text-lg, font-semibold)
  - `Separator`
  - Path info: `Label` + `Code` inline display for paths
  - Two-column comparison layout:
    - "本地版本" card (secondary bg, rounded) with hash, time, view button
    - "远端版本" card (secondary bg, rounded) with hash, time, view button
    - Separator with VS or arrow between them
  - Action buttons row:
    - `Button` variant="default" → "使用本地"
    - `Button` variant="outline" → "使用远端"
    - `Button` variant="ghost" size="sm" → "打开源文件"

**Hash display:**
- `Code` inline or monospace text, text-xs, truncate, max-w-[120px]
- `Tooltip` on hover to show full hash

**Empty State (no conflicts):**
```
+----------------------------------------------------------+
|                                                           |
|                    [CheckCircle icon, 48px, green]        |
|                                                           |
|                    没有待处理的冲突                        |
|                    所有配置文件已同步                       |
|                                                           |
+----------------------------------------------------------+
```
- Centered in the content area
- Icon: `CheckCircle2` (lucide), size=48, text=success
- Title: text-lg, font-medium
- Subtitle: text-sm, muted-foreground

---

## 8. Page 5: Settings

### Layout

```
+----------------------------------------------------------+
|  设置                                                   |
|                                                           |
|  账户信息                                                |
|  +------------------------------------------------------+
|  | GitHub 用户:      ranen                               |
|  | 仓库:             ranen/configpilot-dotfiles  [打开]  |
|  | 仓库类型:         私有 🔒                             |
|  +------------------------------------------------------+
|
|  本地路径                                                |
|  +------------------------------------------------------+
|  | 应用数据目录:     ~/Library/Application Support/...  |
|  |                   [打开目录]                          |
|  | 本地同步仓库:     ~/Library/Application Support/...  |
|  |                   [打开目录]                          |
|  +------------------------------------------------------+
|
|  数据管理                                                |
|  +------------------------------------------------------+
|  | [刷新状态]  [清理本地缓存]                            |
|  |                                                      |
|  | ⚠️ 清理缓存将删除本地工作副本，但不会影响远端仓库       |
|  +------------------------------------------------------+
|
|  同步日志                                                |
|  +------------------------------------------------------+
|  | 2024-01-15 14:30:21  备份完成 (4 个文件)             |
|  | 2024-01-15 14:29:01  连接到 GitHub 仓库              |
|  | ...                                                  |
|  +------------------------------------------------------+
+----------------------------------------------------------+
```

### Components Used

**Section Cards:**
- Each section: title (text-base, font-semibold, mb-3) + `Card`
- Inside card: Key-value rows using `div` with flex layout
  - Label: text-sm, muted-foreground, min-w-[140px]
  - Value: text-sm, font-mono for paths, text-normal for names

**Path Display:**
- Long paths: text-sm, font-mono, break-all or truncate
- "打开目录" / "打开" links: `Button` variant="ghost" size="sm" with `ExternalLink` icon

**Action Buttons:**
- "刷新状态": `Button` variant="outline" with `RefreshCw` icon
- "清理本地缓存": `Button` variant="destructive" with `Trash2` icon
- Before destructive action: inline `Alert` variant="destructive" (subtle) explaining consequences

**Log Display:**
- `Card` with `ScrollArea` (max-h-[240px])
- Same log format as Sync page
- Each line: timestamp + message, spaced apart with py-1

**Loading State:**
- `Skeleton` for each field value while loading

---

## 9. Common Components

### Toast / Notification

- Use shadcn `Toast` + `ToastProvider`
- Position: bottom-right
- Variants: default (info), success (green check), destructive (red x)
- Auto-dismiss after 4 seconds
- Content: icon + title + description

```
+----------------------------------+
| [icon]  备份完成                  |
|         4 个文件已同步到远端仓库    |
+----------------------------------+
```

### Dialog (Modal)

Used for confirmation actions:
- `Dialog` + `DialogTrigger` + `DialogContent` + `DialogHeader` + `DialogFooter`
- Examples: "确认清理缓存？", "确认使用本地版本覆盖远端？"
- Header: title (text-lg, font-semibold) + description (text-sm, muted-foreground)
- Footer: cancel `Button` variant="outline" + confirm `Button` (variant="destructive" for destructive)

### Busy / Loading States

- Button loading: `Loader2` icon spinning inside button, button disabled
- Page loading: `Skeleton` cards (3-4 cards with h-[80px] each)
- Inline loading: `Spinner` (Loader2 with animate-spin) next to text
- Full page overlay: translucent overlay + centered spinner for blocking operations

### MacOS Tray

- Tray icon: 16×16 monochrome app icon (for light/dark menu bar)
- Tray menu items: 显示 ConfigPilot | 立即备份 | 分隔线 | 退出
- On window close → hide to tray (don't quit)

---

## 10. Typography Scale

| Name | Size | Line Height | Usage |
|---|---|---|---|
| text-xs | 12px | 16px | Badges, timestamps, hash |
| text-sm | 14px | 20px | Body, labels, descriptions, table cells |
| text-base | 16px | 24px | Card titles, nav items |
| text-lg | 18px | 28px | Page titles, section headers |
| text-xl | 20px | 28px | Stat values |
| text-2xl | 24px | 32px | Device flow code |

**Font weights:**
- normal (400): body text, descriptions, muted content
- medium (500): labels, table headers, button text
- semibold (600): card titles, nav items, page titles
- bold (700): stat values

---

## 11. Icon Set (lucide-react)

Consistent icon sizing:
- Nav sidebar: 18px
- Card stat icons: 16px
- Button icons: 16px
- Page header icons: 20px
- Empty state: 48px

Icon mapping:
| Context | Icon Name |
|---|---|
| Logo / App | `Pilot` or `Zap` |
| Home nav | `LayoutDashboard` |
| Sync nav | `RefreshCw` |
| Conflicts nav | `GitFork` |
| Settings nav | `Settings` |
| GitHub | `Github` |
| Backup | `Upload` or `ArrowUpToLine` |
| Restore | `Download` or `ArrowDownToLine` |
| Bidirectional | `ArrowLeftRight` |
| Refresh | `RefreshCw` |
| Watch | `Radio` or `Eye` |
| Success | `CheckCircle2` |
| Error | `XCircle` or `AlertTriangle` |
| Warning | `AlertTriangle` |
| Info | `Info` |
| External link | `ExternalLink` |
| Folder | `Folder` or `FolderGit2` |
| File | `FileText` |
| Clock | `Clock` |
| Lock | `Lock` |
| Trash | `Trash2` |
| Spinner | `Loader2` (animate-spin) |

---

## 12. Design Principles for Figma AI

1. **Desktop-first**: Design at 960×640px canvas. No mobile breakpoints. Compact spacing.
2. **shadcn/ui defaults**: Use the default shadcn/ui border-radius (0.5rem = 8px), box-shadows, and transition patterns.
3. **Light mode as primary**: Design the light mode version. Dark mode is a CSS variable swap.
4. **Sidebar navigation**: Fixed left sidebar, never collapsible, always visible.
5. **MacOS native feel**: Subtle shadows, thin borders, system font stack, no rounded mega-corners.
6. **Status-first design**: Every page prominently shows status indicators (dots, badges) so users can assess state at a glance.
7. **Chinese UI copy**: All labels, buttons, and descriptions in Simplified Chinese.
8. **Consistent spacing**: Vertical rhythm of 8px/16px/24px. Cards use padding=16px (p-4). Sections separated by 32px gap.

---

## 13. Component Inventory Summary

Per page, these shadcn/ui components are needed:

| Page | Components |
|---|---|
| Shell | Sidebar (custom), Button, Badge, Separator |
| Home | Card, CardHeader, CardContent, Table, Badge, Button, Tooltip |
| Auth | Card, Button, Alert, Badge, Separator |
| Sync | Card, Button, Table, Checkbox, ScrollArea, Progress, Skeleton, Badge |
| Conflicts | Card, Button, Badge, Alert, Separator, Tooltip, Code (inline) |
| Settings | Card, Button, ScrollArea, Alert, Skeleton, Separator |
| Global | Toast, Dialog, Tooltip, Alert |

---

## 14. Color Assignment Reference

| Element | Light Token | Dark Token |
|---|---|---|
| Page bg | background | background |
| Cards | card | card |
| Sidebar bg | slate-900 (fixed) | slate-900 (fixed) |
| Sidebar text | slate-100 | slate-100 |
| Primary button | primary (zinc-900) | primary (white) |
| Secondary button | secondary (zinc-100) | secondary (slate-700) |
| Outline button | border + background | border + transparent |
| Ghost button | transparent | transparent |
| Table header bg | muted (slate-50) | muted (slate-800) |
| Table border | border | border |
| Success badge | bg-green-100 text-green-700 | bg-green-900/30 text-green-400 |
| Warning badge | bg-amber-100 text-amber-700 | bg-amber-900/30 text-amber-400 |
| Destructive badge | bg-red-100 text-red-700 | bg-red-900/30 text-red-400 |
| Muted text | muted-foreground | muted-foreground |
