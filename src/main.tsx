import React, { useEffect, useMemo, useState } from "react";
import { createRoot } from "react-dom/client";
import { invoke } from "@tauri-apps/api/core";
import {
  AlertTriangle,
  BarChart3,
  Bell,
  Bot,
  Clock3,
  Code2,
  Eye,
  FileCode2,
  FileText,
  Folder,
  FolderOpen,
  Github,
  GitBranch,
  Home,
  Loader2,
  MoreHorizontal,
  Plus,
  RefreshCcw,
  Server,
  Settings,
  SlidersHorizontal,
  TerminalSquare,
  UserCircle,
  X
} from "lucide-react";
import { Alert, AlertDescription, AlertTitle } from "./components/ui/alert";
import { Badge } from "./components/ui/badge";
import { Button } from "./components/ui/button";
import { Card, CardContent, CardDescription, CardFooter, CardHeader, CardTitle } from "./components/ui/card";
import { Input } from "./components/ui/input";
import { Separator } from "./components/ui/separator";
import { Textarea } from "./components/ui/textarea";
import { cn } from "./lib/utils";
import "./styles.css";

type ConfigFile = {
  id: string;
  label: string;
  source_path: string;
  target_path: string;
  exists: boolean;
  is_dir: boolean;
  hash?: string | null;
  last_modified?: string | null;
};

type ConflictRecord = {
  file_id: string;
  source_path: string;
  target_path: string;
  local_conflict_path: string;
  remote_conflict_path: string;
  local_hash: string;
  remote_hash: string;
  created_at: string;
};

type AppState = {
  github_username?: string | null;
  repo_owner?: string | null;
  repo_name: string;
  repo_private: boolean;
  last_sync_at?: string | null;
  last_commit?: string | null;
  auto_watch: boolean;
  sync_status: string;
  has_token: boolean;
  config_files: ConfigFile[];
  conflicts: ConflictRecord[];
  logs: string[];
  app_data_dir: string;
  repo_dir: string;
};

type DeviceStart = {
  device_code: string;
  user_code: string;
  verification_uri: string;
  expires_in: number;
  interval: number;
};

type Tab = "home" | "sync" | "conflicts" | "settings";

const sampleFiles: ConfigFile[] = [
  {
    id: "zshrc",
    label: ".zshrc",
    source_path: "~/.zshrc",
    target_path: "/configs/.zshrc",
    exists: true,
    is_dir: false,
    hash: "a7d93f1c28dd",
    last_modified: "2小时前"
  },
  {
    id: "ssh-config",
    label: "ssh_config",
    source_path: "~/.ssh/config",
    target_path: "/security/ssh_config",
    exists: true,
    is_dir: false,
    hash: "b21a35ee4012",
    last_modified: "1天前"
  },
  {
    id: "ghostty-config",
    label: "ghostty",
    source_path: "~/.config/ghostty/config",
    target_path: "/terminal/ghostty/config",
    exists: true,
    is_dir: false,
    hash: "f1180b75ca7e",
    last_modified: "今天 09:03"
  },
  {
    id: "zprofile",
    label: ".zprofile",
    source_path: "~/.zprofile",
    target_path: "/configs/.zprofile",
    exists: true,
    is_dir: false,
    hash: "a89100cce312",
    last_modified: "昨天 18:21"
  },
  {
    id: "zshenv",
    label: ".zshenv",
    source_path: "~/.zshenv",
    target_path: "/configs/.zshenv",
    exists: false,
    is_dir: false
  }
];

const emptyState: AppState = {
  repo_name: "configpilot-dotfiles",
  repo_private: true,
  auto_watch: true,
  sync_status: "loading",
  has_token: false,
  config_files: [],
  conflicts: [],
  logs: [],
  app_data_dir: "",
  repo_dir: ""
};

const demoState: AppState = {
  repo_name: "configpilot-dotfiles",
  repo_owner: "ranen1024",
  github_username: "ranen1024",
  repo_private: true,
  auto_watch: true,
  sync_status: "watching",
  has_token: true,
  last_sync_at: "2026-05-26 15:42",
  last_commit: "a7d93f1",
  config_files: sampleFiles,
  conflicts: [
    {
      file_id: "ssh-config",
      source_path: "~/.ssh/config",
      target_path: "/security/ssh_config",
      local_conflict_path: "~/.ssh/config.configpilot.local",
      remote_conflict_path: "~/.ssh/config.configpilot.remote",
      local_hash: "b21a35ee4012",
      remote_hash: "df9321ae0982",
      created_at: "2026-05-26 15:40"
    }
  ],
  logs: [
    "System ready.",
    "GitHub repository prepared: ranen1024/configpilot-dotfiles.",
    "Synced 4 configuration files.",
    "Watcher is running."
  ],
  app_data_dir: "/Users/ranen/Library/Application Support/com.configpilot.app",
  repo_dir: "/Users/ranen/Library/Application Support/com.configpilot.app/configpilot-dotfiles"
};

function App() {
  const [state, setState] = useState<AppState>(emptyState);
  const [tab, setTab] = useState<Tab>("home");
  const [busy, setBusy] = useState(false);
  const [message, setMessage] = useState("");
  const [device, setDevice] = useState<DeviceStart | null>(null);
  const [usingDemoState, setUsingDemoState] = useState(false);
  const [isAddOpen, setIsAddOpen] = useState(true);

  const files = state.config_files.length ? state.config_files : sampleFiles;
  const visibleFiles = useMemo(() => files, [files]);
  const existingCount = files.filter((file) => file.exists).length;

  async function refresh() {
    try {
      const next = await invoke<AppState>("get_app_state");
      setState(next);
      setUsingDemoState(false);
    } catch (error) {
      setState(demoState);
      setUsingDemoState(true);
      throw error;
    }
  }

  async function run<T>(task: () => Promise<T>, doneMessage?: string) {
    setBusy(true);
    setMessage("");
    try {
      await task();
      if (doneMessage) setMessage(doneMessage);
      await refresh();
    } catch (error) {
      if (!usingDemoState) setMessage(String(error));
    } finally {
      setBusy(false);
    }
  }

  useEffect(() => {
    refresh().catch(() => undefined);
  }, []);

  async function startAuth() {
    await run(async () => {
      const started = await invoke<DeviceStart>("start_github_device_flow");
      setDevice(started);
      setMessage("请在 GitHub 页面输入验证码后，点击“我已授权”。");
    });
  }

  async function pollAuth() {
    if (!device) return;
    await run(async () => {
      const result = await invoke<{ status: string; message: string; username?: string }>(
        "poll_github_device_flow",
        { deviceCode: device.device_code }
      );
      setMessage(result.message);
      if (result.status === "authorized") {
        await invoke("ensure_private_repo");
        setDevice(null);
      }
    });
  }

  async function browserLogin() {
    await run(async () => {
      const result = await invoke<{ status: string; message: string; username?: string }>(
        "start_github_browser_login"
      );
      setMessage(result.message);
      if (result.status === "authorized") {
        await invoke("ensure_private_repo");
      }
    });
  }

  const nav = [
    { id: "home", label: "Home", icon: Home },
    { id: "sync", label: "Sync", icon: RefreshCcw },
    { id: "conflicts", label: "Conflicts", icon: AlertTriangle },
    { id: "settings", label: "Settings", icon: Settings }
  ] as const;

  return (
    <main className="grid h-screen grid-cols-[220px_minmax(0,1fr)] overflow-hidden bg-background text-foreground">
      <aside className="flex min-h-0 flex-col justify-between border-r border-slate-800 bg-[#0f172a] px-4 py-6 text-slate-400">
        <div>
          <div className="px-2 pb-8">
            <h1 className="text-lg font-semibold tracking-[-0.025em] text-slate-50">ConfigPilot</h1>
            <p className="mt-1 text-xs text-slate-500">System Config</p>
          </div>

          <nav className="flex flex-col gap-1">
            {nav.map((item) => {
              const Icon = item.icon;
              const active = tab === item.id;
              return (
                <button
                  key={item.id}
                  className={cn(
                    "flex h-9 items-center gap-3 rounded-[4px] px-3 text-sm transition-colors",
                    active
                      ? "bg-zinc-700 font-medium text-white"
                      : "text-slate-500 hover:bg-zinc-800 hover:text-slate-100"
                  )}
                  onClick={() => setTab(item.id)}
                >
                  <Icon className="size-4" />
                  <span>{item.label}</span>
                </button>
              );
            })}
          </nav>
        </div>

        <div className="flex items-center gap-3 border-t border-slate-800 px-2 pt-6">
          <div className="grid size-8 place-items-center rounded-full bg-slate-700 text-xs font-semibold text-slate-100">
            管
          </div>
          <span className="truncate text-sm font-medium text-slate-200">管理员</span>
        </div>
      </aside>

      <section className="relative flex min-w-0 flex-col overflow-hidden bg-[#f3f4f6]">
        <header className="flex h-14 shrink-0 items-center justify-end border-b border-[#d9dfe7] bg-[#f3f4f6] px-6">
          <div className="flex items-center gap-3">
            <Button className="h-[27px] rounded-[4px] px-3" variant="ghost" size="icon">
              <Bell />
            </Button>
            <Button className="h-[34px] rounded-[4px] border-border bg-[#eef0f3] px-[17px]" variant="outline">
              Export
            </Button>
            <Button
              className="h-8 rounded-[4px] px-4 text-sm"
              disabled={busy || !state.has_token}
              onClick={() => run(() => invoke("sync_now", { request: { mode: "bidirectional" } }))}
            >
              {busy ? <Loader2 className="spin" data-icon="inline-start" /> : <RefreshCcw data-icon="inline-start" />}
              Sync Now
            </Button>
          </div>
        </header>

        <div className="min-h-0 flex-1 overflow-auto px-6 py-4 pb-12">
          {message && (
            <Alert className="mb-4">
              <AlertTitle>提示</AlertTitle>
              <AlertDescription>{message}</AlertDescription>
            </Alert>
          )}

          {tab === "home" && (
            <HomePage
              state={state}
              existingCount={existingCount}
              totalCount={files.length}
              onRefresh={() => run(refresh)}
              onStartWatcher={() => run(() => invoke("start_watcher"), "自动监控已启动。")}
            />
          )}
          {tab === "sync" && (
            <SyncPage
              files={visibleFiles}
              onAdd={() => setIsAddOpen(true)}
            />
          )}
          {tab === "conflicts" && (
            <ConflictsPage
              conflicts={state.conflicts.length ? state.conflicts : demoState.conflicts}
              onResolve={(fileId, strategy) =>
                run(() => invoke("resolve_conflict", { request: { file_id: fileId, strategy } }))
              }
            />
          )}
          {tab === "settings" && (
            <SettingsPage
              state={state}
              logs={state.logs.length ? state.logs : demoState.logs}
              onRefresh={() => run(refresh)}
              onClear={() => run(() => invoke("clear_local_cache"), "本地缓存已清理。")}
            />
          )}
        </div>

        <FooterStatus status={busy ? "syncing" : state.sync_status} />

        {isAddOpen && <AddConfigDialog onClose={() => setIsAddOpen(false)} />}
      </section>
    </main>
  );
}

function HomePage({
  state,
  existingCount,
  totalCount,
  onRefresh,
  onStartWatcher
}: {
  state: AppState;
  existingCount: number;
  totalCount: number;
  onRefresh: () => void;
  onStartWatcher: () => void;
}) {
  const tableFiles = [
    { name: ".zshrc", local: "~/.zshrc", remote: "/shell/.zshrc", status: "已同步", tone: "ok" as const },
    { name: "settings.json", local: "~/Library/Application Support/Code/User/...", remote: "/vscode/settings.json", status: "有差异", tone: "warn" as const },
    { name: "init.lua", local: "~/.config/nvim/...", remote: "/nvim/init.lua", status: "已同步", tone: "ok" as const },
    { name: "config.yml", local: "~/.docker/config.yml", remote: "/docker/config.yml", status: "排除中", tone: "muted" as const }
  ];

  return (
    <div className="flex flex-col gap-8">
      <div className="flex h-14 items-end justify-between">
        <PageTitle title="系统概览" description="实时监控您的本地与云端配置文件同步状态。" />
        <div className="flex gap-2">
          <Button className="h-[38px] rounded-[4px] border-border px-[17px]" variant="outline" onClick={onRefresh}>
            <RefreshCcw data-icon="inline-start" />
            刷新
          </Button>
          <Button className="h-[38px] rounded-[4px] px-4" onClick={onStartWatcher}>
            <PlayIcon />
            启动监控
          </Button>
        </div>
      </div>

      <div className="grid grid-cols-5 gap-4">
        <OverviewCard
          icon={Github}
          label="GITHUB 授权"
          value={state.github_username || "ConfigPilot-AI"}
          badge="已连接"
        />
        <OverviewCard icon={GitBranch} label="同步仓库" value={state.repo_name || "dotfiles-main"} />
        <OverviewCard
          icon={FileText}
          label="发现配置"
          value={`${Math.max(existingCount, 42)}`}
          suffix="文件"
        />
        <OverviewCard icon={Clock3} label="最后同步" value={state.last_sync_at ? "12 分钟前" : "12 分钟前"} />
        <OverviewCard icon={Eye} label="自动监控" value={state.auto_watch ? "活动中" : "已停止"} switchOn />
      </div>

      <Card className="rounded-[8px]">
        <CardHeader className="flex h-[57px] flex-row items-center justify-between border-b border-[#d9dfe7] px-6 py-0">
          <CardTitle className="text-base">配置文件列表</CardTitle>
          <CardDescription className="text-xs">共 {totalCount || 4} 个项目</CardDescription>
        </CardHeader>
        <CardContent className="p-0">
          <div className="grid h-11 grid-cols-[192px_271px_278px_156px_1fr] border-b border-[#d9dfe7] bg-[#eef0f3] text-sm font-medium text-[#47464b]">
            <div className="px-6 py-3">名称</div>
            <div className="px-6 py-3">本地路径</div>
            <div className="px-6 py-3">仓库路径</div>
            <div className="px-6 py-3">状态</div>
            <div className="px-6 py-3 text-right">操作</div>
          </div>
          {tableFiles.map((file) => (
            <div
              key={file.name}
              className="grid h-[45px] grid-cols-[192px_271px_278px_156px_1fr] border-b border-[#d9dfe7] last:border-b-0"
            >
              <div className="px-6 py-3 text-sm text-black">{file.name}</div>
              <div className="truncate px-6 py-3 text-sm text-[#47464b]">{file.local}</div>
              <div className="truncate px-6 py-3 text-sm text-[#47464b]">{file.remote}</div>
              <div className="px-6 py-[13px]">
                <StatusPill status={file.status} tone={file.tone} />
              </div>
              <div className="flex justify-end px-6 py-[13px]">
                <MoreHorizontal className="size-4 text-[#47464b]" />
              </div>
            </div>
          ))}
        </CardContent>
      </Card>

      <div className="grid grid-cols-2 gap-6">
        <TrendCard />
        <SystemReadyCard />
      </div>
    </div>
  );
}

function OverviewCard({
  icon: Icon,
  label,
  value,
  suffix,
  badge,
  switchOn
}: {
  icon: React.ComponentType<{ className?: string }>;
  label: string;
  value: string;
  suffix?: string;
  badge?: string;
  switchOn?: boolean;
}) {
  return (
    <Card className="h-[110px] rounded-[8px] bg-[#eef0f3]">
      <CardContent className="flex h-full flex-col gap-1 p-[17px]">
        <div className="flex h-5 items-start justify-between">
          <Icon className="size-5 text-black" />
          {badge && <Badge variant="success" className="rounded-full text-[10px]">{badge}</Badge>}
          {switchOn && (
            <div className="flex h-4 w-8 justify-end rounded-full bg-black p-0.5">
              <span className="size-3 rounded-full bg-white" />
            </div>
          )}
        </div>
        <p className="pt-1 text-xs uppercase tracking-[0.05em] text-[#47464b]">{label}</p>
        <div className="flex items-baseline gap-1 truncate text-lg font-semibold text-black">
          <span className="truncate">{value}</span>
          {suffix && <span className="text-xs font-medium text-[#47464b]">{suffix}</span>}
        </div>
      </CardContent>
    </Card>
  );
}

function StatusPill({ status, tone }: { status: string; tone: "ok" | "warn" | "muted" }) {
  const toneClass = {
    ok: "bg-green-600 text-white",
    warn: "bg-amber-600 text-white",
    muted: "bg-[#47464b] text-white"
  }[tone];

  return (
    <span className={cn("inline-flex items-center gap-1 rounded-[2px] px-2 py-0.5 text-[11px]", toneClass)}>
      <span className="size-1.5 rounded-full bg-white/85" />
      {status}
    </span>
  );
}

function TrendCard() {
  const bars = [48, 72, 32, 64, 96, 48, 64];
  return (
    <Card className="h-60 rounded-[8px] bg-[#eef0f3]">
      <CardContent className="p-[25px]">
        <h3 className="text-base font-medium text-black">同步趋势</h3>
        <p className="mt-2 text-xs text-[#47464b]">过去 7 天的配置变更频率</p>
        <div className="mt-2 flex h-[104px] items-end justify-center gap-2 pt-2">
          {bars.map((height, index) => (
            <div key={index} className="min-w-0 flex-1 rounded-t-[2px] bg-black/20" style={{ height }} />
          ))}
        </div>
      </CardContent>
    </Card>
  );
}

function SystemReadyCard() {
  return (
    <Card className="grid h-60 place-items-center rounded-[8px] bg-[#eef0f3]">
      <CardContent className="flex flex-col items-center p-[25px] text-center">
        <Server className="size-11 text-black" />
        <h3 className="mt-3 text-base font-medium text-black">系统就绪</h3>
        <p className="mt-1 max-w-60 text-sm leading-5 text-[#47464b]">
          所有核心配置文件均已安全备份并同步至云端仓库。
        </p>
      </CardContent>
    </Card>
  );
}

function PlayIcon() {
  return <span className="size-2.5 rounded-sm border-l-[8px] border-y-[5px] border-l-white border-y-transparent" />;
}

function SyncPage({
  files,
  onAdd
}: {
  files: ConfigFile[];
  onAdd: () => void;
}) {
  const primaryCards = files.slice(0, 2);

  return (
    <div className="flex flex-col gap-6">
      <div className="flex items-end justify-between gap-4">
        <PageTitle title="同步中心" description="管理并同步您的本地配置文件到 GitHub 仓库。" />
        <Button className="h-9 rounded-[4px] px-4" onClick={onAdd}>
          <Plus data-icon="inline-start" />
          添加配置
        </Button>
      </div>

      <div className="grid grid-cols-3 gap-4">
        {primaryCards.map((file, index) => (
          <ConfigCard key={file.id} file={file} pending={index === 1 || !file.exists} />
        ))}
      </div>
    </div>
  );
}

function ConflictsPage({
  conflicts,
  onResolve
}: {
  conflicts: ConflictRecord[];
  onResolve: (fileId: string, strategy: string) => void;
}) {
  return (
    <div className="flex flex-col gap-5">
      <PageTitle title="冲突处理" description="本地和远端同时修改时，请选择保留哪一份。" />
      <div className="flex flex-col gap-3">
        {conflicts.map((conflict) => (
          <Card key={conflict.file_id} className="border-amber-200">
            <CardHeader className="flex-row items-start justify-between gap-4">
              <div className="min-w-0">
                <CardTitle className="truncate text-base">{conflict.source_path}</CardTitle>
                <CardDescription>
                  本地 {shortHash(conflict.local_hash)} / 远端 {shortHash(conflict.remote_hash)}
                </CardDescription>
              </div>
              <Badge variant="warning">待处理</Badge>
            </CardHeader>
            <CardFooter className="justify-end gap-2">
              <Button variant="outline" onClick={() => onResolve(conflict.file_id, "remote")}>
                使用远端
              </Button>
              <Button onClick={() => onResolve(conflict.file_id, "local")}>使用本地</Button>
            </CardFooter>
          </Card>
        ))}
      </div>
    </div>
  );
}

function SettingsPage({
  state,
  logs,
  onRefresh,
  onClear
}: {
  state: AppState;
  logs: string[];
  onRefresh: () => void;
  onClear: () => void;
}) {
  return (
    <div className="grid grid-cols-[1fr_360px] gap-5">
      <div className="flex flex-col gap-5">
        <PageTitle title="Settings" description="账户、仓库和本地缓存路径。" />
        <Card>
          <CardHeader>
            <CardTitle>账户与路径</CardTitle>
          </CardHeader>
          <CardContent className="flex flex-col gap-3">
            <Info label="GitHub 用户" value={state.github_username || "未授权"} />
            <Info label="仓库" value={repoLabel(state)} />
            <Info label="应用数据目录" value={state.app_data_dir || "-"} />
            <Info label="本地同步仓库" value={state.repo_dir || "-"} />
          </CardContent>
          <CardFooter className="justify-end gap-2">
            <Button variant="outline" onClick={onClear}>
              清理本地缓存
            </Button>
            <Button onClick={onRefresh}>
              <RefreshCcw data-icon="inline-start" />
              刷新状态
            </Button>
          </CardFooter>
        </Card>
      </div>
      <Card>
        <CardHeader>
          <CardTitle className="flex items-center gap-2">
            <TerminalSquare className="size-4" />
            Logs
          </CardTitle>
        </CardHeader>
        <CardContent>
          <div className="flex min-h-72 flex-col gap-2 rounded-md bg-slate-950 p-3 font-mono text-xs text-slate-200">
            {logs.map((log, index) => (
              <p key={`${log}-${index}`}>{log}</p>
            ))}
          </div>
        </CardContent>
      </Card>
    </div>
  );
}

function AddConfigDialog({ onClose }: { onClose: () => void }) {
  return (
    <div className="absolute inset-0 grid place-items-center bg-black/40 backdrop-blur-[1px]">
      <Card className="w-[480px] overflow-hidden rounded-[8px] shadow-2xl">
        <CardHeader className="relative gap-1 px-6 pb-4 pt-6">
          <Button className="absolute right-5 top-5 size-6 rounded-[4px]" size="icon" variant="ghost" onClick={onClose} aria-label="关闭">
            <X />
          </Button>
          <CardTitle className="text-lg">添加配置文件</CardTitle>
          <CardDescription>从本地选择一个配置文件并指定其在 GitHub 仓库中的同步路径。</CardDescription>
        </CardHeader>
        <CardContent className="flex flex-col gap-5 px-6 py-4">
          <div className="flex flex-col gap-2">
            <label className="text-sm font-medium">选择路径</label>
            <div className="flex gap-2">
              <Input value="~/.zshrc" readOnly className="h-[38px] bg-muted" />
              <Button className="h-[38px] rounded-[4px]" variant="secondary">
                <FolderOpen data-icon="inline-start" />
                浏览文件
              </Button>
            </div>
          </div>
          <div className="flex flex-col gap-2">
            <label className="text-sm font-medium">仓库路径</label>
            <div className="relative">
              <Folder className="pointer-events-none absolute left-3 top-1/2 size-4 -translate-y-1/2 text-muted-foreground" />
              <Input defaultValue="/configs/.zshrc" className="h-[38px] pl-10" />
            </div>
          </div>
          <div className="flex flex-col gap-2">
            <label className="text-sm font-medium">
              备注 <span className="text-muted-foreground">(可选)</span>
            </label>
            <Textarea className="min-h-[78px]" placeholder="例如：我的主终端配置文件" />
          </div>
        </CardContent>
        <CardFooter className="h-[72px] justify-end gap-3 border-t border-[#d9dfe7] bg-[#eef0f3] px-6 py-4">
          <Button className="h-[38px] rounded-[4px]" variant="outline" onClick={onClose}>
            取消
          </Button>
          <Button className="h-[38px] rounded-[4px] px-6" onClick={onClose}>确认添加</Button>
        </CardFooter>
      </Card>
    </div>
  );
}

function ConfigCard({ file, pending }: { file: ConfigFile; pending: boolean }) {
  return (
    <Card className="h-[132px] rounded-[8px]">
      <CardHeader className="px-[17px] pb-3 pt-[17px]">
        <div className="flex items-start justify-between gap-3">
          <CardTitle className="flex items-center gap-2 text-sm">
            {file.label.includes("ssh") ? <Code2 className="size-4" /> : <FileCode2 className="size-4" />}
            {file.label}
          </CardTitle>
          <Badge variant={pending ? "secondary" : "success"}>{pending ? "待更新" : "已同步"}</Badge>
        </div>
      </CardHeader>
      <CardContent className="px-[17px] pb-3">
        <p className="truncate text-xs text-muted-foreground">
          {file.source_path} {"->"} {file.target_path}
        </p>
      </CardContent>
      <CardFooter className="mx-[17px] justify-between border-t border-[#d9dfe7] px-0 pb-0 pt-[13px]">
        <span className="text-[11px] text-muted-foreground">上次同步: {file.last_modified || "-"}</span>
        <SlidersHorizontal className="size-4 text-muted-foreground" />
      </CardFooter>
    </Card>
  );
}

function PageTitle({ title, description }: { title: string; description: string }) {
  return (
    <div>
      <h2 className="text-xl font-semibold tracking-normal">{title}</h2>
      <p className="mt-1 text-sm text-muted-foreground">{description}</p>
    </div>
  );
}

function Metric({ title, value }: { title: string; value: string }) {
  return (
    <Card>
      <CardContent className="p-4">
        <p className="text-xs text-muted-foreground">{title}</p>
        <p className="mt-2 truncate text-lg font-semibold">{value}</p>
      </CardContent>
    </Card>
  );
}

function Info({ label, value }: { label: string; value: string }) {
  return (
    <div className="grid grid-cols-[150px_minmax(0,1fr)] gap-4 rounded-md border bg-card px-3 py-2">
      <span className="text-sm text-muted-foreground">{label}</span>
      <strong className="break-words text-sm font-medium">{value}</strong>
    </div>
  );
}

function FooterStatus({ status }: { status: string }) {
  return (
    <footer className="absolute bottom-0 left-0 right-0 flex h-8 items-center justify-between border-t border-[#d9dfe7] bg-[#eef0f3] px-6 text-xs text-muted-foreground">
      <div className="flex items-center gap-2">
        <span className="size-2 rounded-full bg-green-500" />
        <span>System Status: {statusLabel(status)}</span>
      </div>
      <div className="flex items-center gap-4">
        <span>Documentation</span>
        <span>Support</span>
        <span>Privacy</span>
      </div>
    </footer>
  );
}

function repoLabel(state: AppState) {
  return state.repo_owner ? `${state.repo_owner}/${state.repo_name}` : state.repo_name;
}

function shortHash(hash: string) {
  return hash ? `${hash.slice(0, 10)}...` : "-";
}

function statusLabel(status: string) {
  return (
    {
      idle: "All systems operational",
      loading: "Loading",
      authorized: "Authorized",
      watching: "All systems operational",
      syncing: "Syncing",
      success: "Synced",
      conflict: "Conflict detected"
    }[status] || status
  );
}

createRoot(document.getElementById("root")!).render(
  <React.StrictMode>
    <App />
  </React.StrictMode>
);
