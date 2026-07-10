import { listen, type UnlistenFn } from "@tauri-apps/api/event";
import { open, save } from "@tauri-apps/plugin-dialog";
import { openUrl } from "@tauri-apps/plugin-opener";
import {
  Activity,
  ArrowLeft,
  ArrowUpRight,
  Check,
  ChevronRight,
  CircleStop,
  Clock3,
  Cpu,
  Download,
  FileUp,
  History,
  LibraryBig,
  LoaderCircle,
  Menu,
  PlugZap,
  RefreshCw,
  Search,
  ShieldCheck,
  SlidersHorizontal,
  TerminalSquare,
  Usb,
  X,
  Zap,
} from "lucide-react";
import { useCallback, useEffect, useMemo, useRef, useState } from "react";
import {
  analyzeFirmware,
  downloadOfficialFirmware,
  flashFirmware,
  getCatalog,
  getHistory,
  isDesktop,
  requestUpdateMode,
  saveTranscript,
  scanDevices,
  startConsole,
  stopConsole,
} from "./backend";
import type {
  CatalogItem,
  FirmwareAnalysis,
  FlashEvent,
  FlashResult,
  HistoryEntry,
  TargetProfile,
  TouchDevice,
} from "./types";

type Page = "library" | "device" | "console" | "history";
type InstallStage = "guide" | "review" | "flashing" | "result";

const profileNames: Record<TargetProfile, string> = {
  "touch2-stm-internal-v1": "Internal flash",
  "touch2-daisy-sram-v1": "Daisy · SRAM",
  "touch2-daisy-qspi-v1": "Daisy · QSPI",
};

const profileAddresses: Record<TargetProfile, string> = {
  "touch2-stm-internal-v1": "0x08000000",
  "touch2-daisy-sram-v1": "0x90040000",
  "touch2-daisy-qspi-v1": "0x90040000",
};

function App() {
  const [page, setPage] = useState<Page>("library");
  const [catalog, setCatalog] = useState<CatalogItem[]>([]);
  const [devices, setDevices] = useState<TouchDevice[]>([]);
  const [history, setHistory] = useState<HistoryEntry[]>([]);
  const [selected, setSelected] = useState<CatalogItem | null>(null);
  const [query, setQuery] = useState("");
  const [filter, setFilter] = useState("all");
  const [installing, setInstalling] = useState<CatalogItem | null>(null);
  const [loading, setLoading] = useState(true);
  const [downloadingId, setDownloadingId] = useState<string | null>(null);
  const [error, setError] = useState<string | null>(null);

  const refreshDevices = useCallback(async () => {
    try {
      setDevices(await scanDevices());
    } catch (reason) {
      setError(String(reason));
    }
  }, []);

  const refreshHistory = useCallback(async () => {
    try {
      setHistory(await getHistory());
    } catch (reason) {
      setError(String(reason));
    }
  }, []);

  useEffect(() => {
    Promise.all([getCatalog(), scanDevices(), getHistory()])
      .then(([items, foundDevices, entries]) => {
        setCatalog(items);
        setDevices(foundDevices);
        setHistory(entries);
        setSelected(items.find((item) => item.featured) ?? items[0] ?? null);
      })
      .catch((reason) => setError(String(reason)))
      .finally(() => setLoading(false));
  }, []);

  useEffect(() => {
    if (!isDesktop) return;
    const timer = window.setInterval(refreshDevices, 1800);
    return () => window.clearInterval(timer);
  }, [refreshDevices]);

  const filtered = useMemo(() => {
    const normalized = query.trim().toLowerCase();
    return catalog.filter((item) => {
      const matchesFilter =
        filter === "all" ||
        (filter === "official" && item.trust === "official") ||
        (filter === "community" && item.trust !== "official") ||
        item.category.toLowerCase() === filter;
      const matchesQuery =
        !normalized ||
        [item.name, item.author, item.category, item.summary, ...item.tags]
          .join(" ")
          .toLowerCase()
          .includes(normalized);
      return matchesFilter && matchesQuery;
    });
  }, [catalog, filter, query]);

  const runtimeDevice = devices.find((device) => device.state === "runtime");
  const dfuDevice = devices.find((device) => device.state !== "runtime");

  async function importFirmware() {
    if (!isDesktop) return;
    const path = await open({
      title: "Add firmware",
      multiple: false,
      filters: [{ name: "Touch 2 firmware", extensions: ["bin"] }],
    });
    if (!path) return;
    try {
      const analysis = await analyzeFirmware(path);
      const localItem: CatalogItem = {
        id: `local-${analysis.sha256.slice(0, 12)}`,
        name: analysis.filename.replace(/\.bin$/i, ""),
        version: "Local",
        author: "Local firmware",
        channel: "local",
        trust: "local",
        category: "Imported",
        summary: "A locally imported binary. Review the inferred target before installing.",
        description:
          "Touch Manager analyzed this raw binary locally. It is not signed or reviewed by the catalog maintainers.",
        tags: [analysis.executionLayout.replace("boot_", "")],
        tone: "black",
        featured: false,
        targetProfile: analysis.inferredProfile ?? "touch2-stm-internal-v1",
        sha256: analysis.sha256,
        artifactPath: path,
        sourceUrl: "",
        runtimeUsb: false,
        managerCompatible: false,
        localPath: path,
        availableLocally: true,
        checksumMatches: true,
        analysis,
      };
      setCatalog((items) => [localItem, ...items]);
      setSelected(localItem);
      setFilter("all");
    } catch (reason) {
      setError(String(reason));
    }
  }

  async function downloadFirmware(item: CatalogItem) {
    setDownloadingId(item.id);
    setError(null);
    try {
      await downloadOfficialFirmware(item.id);
      const refreshed = await getCatalog();
      setCatalog(refreshed);
      setSelected(refreshed.find((candidate) => candidate.id === item.id) ?? null);
    } catch (reason) {
      setError(String(reason));
    } finally {
      setDownloadingId(null);
    }
  }

  return (
    <div className="app-shell">
      <header className="topbar">
        <button className="brand" onClick={() => setPage("library")}>
          <span className="brand-mark" aria-hidden="true">
            <i />
            <i />
            <i />
          </span>
          <span>
            <strong>Touch</strong>
            <small>Manager</small>
          </span>
        </button>

        <nav className="primary-nav" aria-label="Main navigation">
          <NavButton icon={<LibraryBig />} active={page === "library"} onClick={() => setPage("library")}>
            Library
          </NavButton>
          <NavButton icon={<Usb />} active={page === "device"} onClick={() => setPage("device")}>
            Device
          </NavButton>
          <NavButton icon={<TerminalSquare />} active={page === "console"} onClick={() => setPage("console")}>
            Console
          </NavButton>
          <NavButton icon={<History />} active={page === "history"} onClick={() => setPage("history")}>
            History
          </NavButton>
        </nav>

        <DevicePill runtime={runtimeDevice} dfu={dfuDevice} onClick={() => setPage("device")} />
      </header>

      {!isDesktop && (
        <div className="preview-banner">
          Interface preview · USB and filesystem actions are available in the desktop build
        </div>
      )}

      {error && (
        <div className="error-banner" role="alert">
          <span>{error}</span>
          <button onClick={() => setError(null)} aria-label="Dismiss error"><X /></button>
        </div>
      )}

      <main>
        {loading ? (
          <div className="loading-screen"><LoaderCircle className="spin" /> Opening the library…</div>
        ) : page === "library" ? (
          <LibraryPage
            items={filtered}
            selected={selected}
            query={query}
            filter={filter}
            deviceConnected={Boolean(runtimeDevice || dfuDevice)}
            onQuery={setQuery}
            onFilter={setFilter}
            onSelect={setSelected}
            onInstall={setInstalling}
            onDownload={downloadFirmware}
            downloadingId={downloadingId}
            onImport={importFirmware}
          />
        ) : page === "device" ? (
          <DevicePage devices={devices} onRefresh={refreshDevices} onLibrary={() => setPage("library")} />
        ) : page === "console" ? (
          <ConsolePage devices={devices} />
        ) : (
          <HistoryPage entries={history} onLibrary={() => setPage("library")} />
        )}
      </main>

      {installing && (
        <InstallModal
          item={installing}
          devices={devices}
          onClose={() => setInstalling(null)}
          onRefreshDevices={refreshDevices}
          onComplete={async () => {
            await refreshHistory();
            await refreshDevices();
          }}
        />
      )}
    </div>
  );
}

function NavButton({
  icon,
  active,
  onClick,
  children,
}: {
  icon: React.ReactNode;
  active: boolean;
  onClick: () => void;
  children: React.ReactNode;
}) {
  return (
    <button className={active ? "nav-button active" : "nav-button"} onClick={onClick}>
      {icon}<span>{children}</span>
    </button>
  );
}

function ExternalLink({
  href,
  className,
  children,
}: {
  href: string;
  className?: string;
  children: React.ReactNode;
}) {
  return (
    <a
      className={className}
      href={href}
      target="_blank"
      rel="noreferrer"
      onClick={(event) => {
        if (!isDesktop) return;
        event.preventDefault();
        void openUrl(href);
      }}
    >
      {children}
    </a>
  );
}

function DevicePill({
  runtime,
  dfu,
  onClick,
}: {
  runtime?: TouchDevice;
  dfu?: TouchDevice;
  onClick: () => void;
}) {
  const connected = runtime ?? dfu;
  return (
    <button className={connected ? "device-pill connected" : "device-pill"} onClick={onClick}>
      <span className="status-dot" />
      <span>
        <small>{connected ? (runtime ? "TOUCH 2" : "UPDATE MODE") : "TOUCH 2"}</small>
        <strong>{connected ? (runtime ? "Connected" : "Ready to install") : "Not connected"}</strong>
      </span>
      <ChevronRight />
    </button>
  );
}

function LibraryPage({
  items,
  selected,
  query,
  filter,
  deviceConnected,
  onQuery,
  onFilter,
  onSelect,
  onInstall,
  onDownload,
  downloadingId,
  onImport,
}: {
  items: CatalogItem[];
  selected: CatalogItem | null;
  query: string;
  filter: string;
  deviceConnected: boolean;
  onQuery: (value: string) => void;
  onFilter: (value: string) => void;
  onSelect: (item: CatalogItem) => void;
  onInstall: (item: CatalogItem) => void;
  onDownload: (item: CatalogItem) => void;
  downloadingId: string | null;
  onImport: () => void;
}) {
  return (
    <div className="library-layout">
      <section className="library-main">
        <div className="page-heading">
          <div>
            <p className="eyebrow">Instrument library</p>
            <h1>Change the instrument.<br />Keep the hardware.</h1>
          </div>
          <button className="secondary-action" onClick={onImport} disabled={!isDesktop}>
            <FileUp /> Add firmware
          </button>
        </div>

        <div className="library-tools">
          <label className="search-field">
            <Search />
            <input value={query} onChange={(event) => onQuery(event.target.value)} placeholder="Search instruments" />
          </label>
          <div className="filter-row">
            {["all", "official", "community", "synth", "effect", "drone"].map((value) => (
              <button key={value} className={filter === value ? "filter active" : "filter"} onClick={() => onFilter(value)}>
                {value}
              </button>
            ))}
          </div>
        </div>

        <div className="catalog-count"><span>{items.length} instruments</span><SlidersHorizontal /></div>
        <div className="firmware-grid">
          {items.map((item) => (
            <FirmwareCard
              key={item.id}
              item={item}
              selected={selected?.id === item.id}
              onSelect={() => onSelect(item)}
            />
          ))}
        </div>
        <ExternalLink
          className="community-directory"
          href="https://github.com/Synthux-Academy/awesome-synthux#simple-touch-2"
        >
          <span><small>Community directory</small><strong>Explore more Touch 2 instruments</strong></span>
          <ArrowUpRight />
        </ExternalLink>
        {items.length === 0 && <div className="empty-state">No instruments match this view.</div>}
      </section>

      <aside className="detail-panel">
        {selected ? (
          <FirmwareDetail
            item={selected}
            deviceConnected={deviceConnected}
            downloading={downloadingId === selected.id}
            onInstall={() => onInstall(selected)}
            onDownload={() => onDownload(selected)}
          />
        ) : (
          <div className="empty-detail"><Menu /> Select an instrument</div>
        )}
      </aside>
    </div>
  );
}

function FirmwareCard({ item, selected, onSelect }: { item: CatalogItem; selected: boolean; onSelect: () => void }) {
  return (
    <button className={selected ? "firmware-card selected" : "firmware-card"} onClick={onSelect}>
      <div className={`firmware-art tone-${item.tone}`}>
        <span className="art-ring one" /><span className="art-ring two" /><span className="art-line" />
        <small>{item.category}</small>
        <strong>{item.name.slice(0, 2).toUpperCase()}</strong>
      </div>
      <div className="card-copy">
        <div className="card-meta">
          <span className={`trust trust-${item.trust}`}>{item.trust === "official" ? "Official" : "Community"}</span>
          {item.availableLocally && <span className="cached"><Check /> Cached</span>}
        </div>
        <h3>{item.name}</h3>
        <p>{item.summary}</p>
        <div className="card-footer"><span>{item.author}</span><ChevronRight /></div>
      </div>
    </button>
  );
}

function FirmwareDetail({
  item,
  deviceConnected,
  downloading,
  onInstall,
  onDownload,
}: {
  item: CatalogItem;
  deviceConnected: boolean;
  downloading: boolean;
  onInstall: () => void;
  onDownload: () => void;
}) {
  const size = item.analysis ? `${Math.round(item.analysis.size / 1024)} KB` : "—";
  const downloadable =
    item.trust === "official" && item.license === "MIT" && Boolean(item.downloadUrl);
  return (
    <div className="detail-content">
      <div className={`detail-hero tone-${item.tone}`}>
        <span className="detail-number">{item.name.slice(0, 2).toUpperCase()}</span>
        <div><small>{item.category}</small><strong>{item.version}</strong></div>
      </div>
      <div className="detail-title">
        <p className="eyebrow">{item.trust === "official" ? "Synthux official" : "Community instrument"}</p>
        <h2>{item.name}</h2>
        <p>{item.description}</p>
      </div>
      <div className="tag-row">{item.tags.map((tag) => <span key={tag}>{tag}</span>)}</div>
      <dl className="firmware-facts">
        <div><dt>Target</dt><dd>{profileNames[item.targetProfile]}</dd></div>
        <div><dt>Binary</dt><dd>{size}</dd></div>
        <div><dt>Update return</dt><dd>{item.managerCompatible ? "Automatic" : "Manual recovery"}</dd></div>
        <div><dt>Checksum</dt><dd className={item.checksumMatches === false ? "bad" : ""}>{item.checksumMatches === false ? "Mismatch" : item.availableLocally ? "Verified" : "Not cached"}</dd></div>
      </dl>
      <div className="detail-actions">
        {item.availableLocally ? (
          <button className="install-action" onClick={onInstall} disabled={item.checksumMatches === false || !isDesktop}>
            <Download />
            <span><small>{deviceConnected ? "Ready when you are" : "Connect Touch 2"}</small>Install instrument</span>
          </button>
        ) : downloadable ? (
          <button className="install-action download-action" onClick={onDownload} disabled={downloading || !isDesktop}>
            {downloading ? <LoaderCircle className="spin" /> : <Download />}
            <span><small>Verified official release</small>{downloading ? "Downloading…" : "Download firmware"}</span>
          </button>
        ) : null}
        {!item.availableLocally && !downloadable && item.trust === "official" && (
          <p className="action-note">This official binary is not mirrored because its upstream redistribution license is not confirmed.</p>
        )}
        {item.trust !== "official" && (
          <ExternalLink className="community-source" href="https://github.com/Synthux-Academy/awesome-synthux#simple-touch-2">
            Browse the Touch 2 community directory <ArrowUpRight />
          </ExternalLink>
        )}
        {item.sourceUrl && <ExternalLink href={item.sourceUrl}>View source <ArrowUpRight /></ExternalLink>}
      </div>
    </div>
  );
}

function DevicePage({ devices, onRefresh, onLibrary }: { devices: TouchDevice[]; onRefresh: () => void; onLibrary: () => void }) {
  const device = devices[0];
  return (
    <div className="single-page">
      <div className="page-heading compact">
        <div><p className="eyebrow">Hardware</p><h1>Your Touch 2</h1></div>
        <button className="secondary-action" onClick={onRefresh}><RefreshCw /> Scan again</button>
      </div>
      {device ? (
        <div className="device-dashboard">
          <section className="device-hero-card">
            <div className="device-silhouette"><span /><span /><span /><span /></div>
            <div>
              <span className="live-badge"><i /> Connected</span>
              <h2>{device.state === "runtime" ? "Touch 2" : "Touch 2 · Update mode"}</h2>
              <p>{device.product ?? "STM32 / Daisy device"}</p>
            </div>
          </section>
          <section className="info-card">
            <h3><Usb /> Connection</h3>
            <dl>
              <div><dt>State</dt><dd>{device.state.replaceAll("_", " ")}</dd></div>
              <div><dt>USB identity</dt><dd>{device.vendorId}:{device.productId}</dd></div>
              <div><dt>Port</dt><dd>{device.serialPort ?? "DFU interface"}</dd></div>
              <div><dt>Topology</dt><dd>{device.topologyPath}</dd></div>
            </dl>
          </section>
          <section className="info-card">
            <h3><ShieldCheck /> Recovery</h3>
            <p>The factory STM32 recovery cannot be erased by an ordinary firmware install.</p>
            <ol><li>Hold BOOT</li><li>Tap RESET</li><li>Release BOOT</li></ol>
          </section>
          <button className="wide-cta" onClick={onLibrary}><LibraryBig /> Choose another instrument <ChevronRight /></button>
        </div>
      ) : (
        <div className="connect-empty">
          <div className="cable-illustration"><PlugZap /></div>
          <p className="eyebrow">No device found</p>
          <h2>Connect Touch 2 with a data-capable USB cable.</h2>
          <p>The app will recognize normal runtime mode and both supported DFU modes.</p>
          <button className="primary-action" onClick={onRefresh}><RefreshCw /> Scan for Touch 2</button>
        </div>
      )}
    </div>
  );
}

function ConsolePage({ devices }: { devices: TouchDevice[] }) {
  const runtime = devices.find((device) => device.serialPort);
  const [session, setSession] = useState<string | null>(null);
  const [lines, setLines] = useState<{ time: string; line: string }[]>([]);
  const outputRef = useRef<HTMLDivElement>(null);

  useEffect(() => {
    if (!isDesktop) return;
    let dispose: UnlistenFn | undefined;
    listen<{ sessionId: string; line: string }>("serial-line", (event) => {
      setLines((current) => [...current.slice(-999), { time: new Date().toLocaleTimeString(), line: event.payload.line }]);
    }).then((unlisten) => (dispose = unlisten));
    return () => dispose?.();
  }, []);

  useEffect(() => {
    outputRef.current?.scrollTo({ top: outputRef.current.scrollHeight });
  }, [lines]);

  async function toggle() {
    if (session) {
      await stopConsole(session);
      setSession(null);
    } else if (runtime?.serialPort) {
      setSession(await startConsole(runtime.serialPort));
    }
  }

  async function exportLog() {
    if (!isDesktop) return;
    const path = await save({ defaultPath: "touch-manager-console.txt", filters: [{ name: "Text", extensions: ["txt"] }] });
    if (path) await saveTranscript(path, lines.map((entry) => `${entry.time}  ${entry.line}`).join("\n"));
  }

  return (
    <div className="single-page console-page">
      <div className="page-heading compact">
        <div><p className="eyebrow">Diagnostics</p><h1>Device console</h1></div>
        <div className="heading-actions">
          <button className="secondary-action" onClick={exportLog} disabled={!lines.length}><Download /> Export</button>
          <button className={session ? "stop-action" : "primary-action"} onClick={toggle} disabled={!runtime?.serialPort || !isDesktop}>
            {session ? <CircleStop /> : <Activity />}{session ? "Stop listening" : "Start listening"}
          </button>
        </div>
      </div>
      <div className="console-status">
        <span className={session ? "console-light active" : "console-light"} />
        <span>{runtime?.serialPort ?? "No USB serial port"}</span>
        <small>115200 baud</small>
      </div>
      <div className="console-output" ref={outputRef}>
        {lines.length ? lines.map((entry, index) => (
          <div className="console-line" key={`${entry.time}-${index}`}><time>{entry.time}</time><span>{entry.line}</span></div>
        )) : (
          <div className="console-placeholder">
            <TerminalSquare />
            <h3>No diagnostic messages yet</h3>
            <p>Compatible firmware can stream touch values, setup state, and structured fault information here.</p>
          </div>
        )}
      </div>
    </div>
  );
}

function HistoryPage({ entries, onLibrary }: { entries: HistoryEntry[]; onLibrary: () => void }) {
  return (
    <div className="single-page">
      <div className="page-heading compact"><div><p className="eyebrow">Audit trail</p><h1>Install history</h1></div></div>
      {entries.length ? (
        <div className="history-list">
          {entries.map((entry) => (
            <article key={entry.id} className="history-entry">
              <span className={`history-icon ${entry.status}`}><Check /></span>
              <div><h3>{entry.firmwareName}</h3><p>{entry.version} · {entry.targetProfile.replaceAll("_", " ")}</p></div>
              <code>{entry.sha256.slice(0, 12)}</code>
              <time>{new Date(entry.createdAt).toLocaleString()}</time>
              <span className={`history-status ${entry.status}`}>{entry.status.replaceAll("_", " ")}</span>
            </article>
          ))}
        </div>
      ) : (
        <div className="connect-empty small"><Clock3 /><h2>No installations recorded yet.</h2><p>Completed and recovery-required attempts will appear here.</p><button className="primary-action" onClick={onLibrary}>Open library</button></div>
      )}
    </div>
  );
}

function InstallModal({
  item,
  devices,
  onClose,
  onRefreshDevices,
  onComplete,
}: {
  item: CatalogItem;
  devices: TouchDevice[];
  onClose: () => void;
  onRefreshDevices: () => Promise<void>;
  onComplete: () => Promise<void>;
}) {
  const needsDaisyBootloader = item.targetProfile !== "touch2-stm-internal-v1";
  const hasCompatibleDfu = devices.some((device) =>
    needsDaisyBootloader
      ? device.state === "daisy_bootloader"
      : device.state === "stm_rom_dfu",
  );
  const incompatibleDfu = devices.find((device) =>
    device.state !== "runtime" &&
    (needsDaisyBootloader
      ? device.state !== "daisy_bootloader"
      : device.state !== "stm_rom_dfu"),
  );
  const runtime = devices.find((device) => device.state === "runtime");
  const [stage, setStage] = useState<InstallStage>(hasCompatibleDfu ? "review" : "guide");
  const [confirmed, setConfirmed] = useState(false);
  const [event, setEvent] = useState<FlashEvent>({ phase: "idle", message: "Ready" });
  const [result, setResult] = useState<FlashResult | null>(null);
  const [modalError, setModalError] = useState<string | null>(null);

  useEffect(() => {
    if (hasCompatibleDfu && stage === "guide") setStage("review");
  }, [hasCompatibleDfu, stage]);

  useEffect(() => {
    if (!isDesktop) return;
    let dispose: UnlistenFn | undefined;
    listen<FlashEvent>("flash-event", (incoming) => setEvent(incoming.payload)).then((unlisten) => (dispose = unlisten));
    return () => dispose?.();
  }, []);

  async function askForUpdateMode() {
    if (!runtime?.serialPort) return;
    try {
      await requestUpdateMode(runtime.serialPort);
      await onRefreshDevices();
    } catch (reason) {
      setModalError(`This firmware did not accept the update request. Use the buttons below. ${String(reason)}`);
    }
  }

  async function install() {
    if (!item.localPath) return;
    setStage("flashing");
    setModalError(null);
    try {
      const response = await flashFirmware({
        firmwareId: item.id,
        firmwareName: item.name,
        version: item.version,
        path: item.localPath,
        expectedSha256: item.sha256,
        targetProfile: item.targetProfile,
        expectRuntime: item.runtimeUsb,
        confirmed,
      });
      setResult(response);
      setStage("result");
      await onComplete();
    } catch (reason) {
      setModalError(String(reason));
      setStage("review");
    }
  }

  return (
    <div className="modal-backdrop" role="presentation">
      <section className="install-modal" role="dialog" aria-modal="true" aria-label={`Install ${item.name}`}>
        <header>
          <button className="icon-button" onClick={stage === "guide" ? onClose : () => setStage("guide")} aria-label={stage === "guide" ? "Close" : "Back"}>
            {stage === "guide" ? <X /> : <ArrowLeft />}
          </button>
          <div><small>Install instrument</small><strong>{item.name}</strong></div>
          <span className="step-label">{stage === "guide" ? "1 / 3" : stage === "review" ? "2 / 3" : "3 / 3"}</span>
        </header>

        {modalError && <div className="modal-error">{modalError}</div>}

        {stage === "guide" && (
          <div className="guide-stage">
            <p className="eyebrow">Enter update mode</p>
            <h2>{needsDaisyBootloader ? "Wake the Daisy Bootloader." : "Two buttons. One quick gesture."}</h2>
            <p>Touch Manager is already listening and will continue automatically when the device appears.</p>
            {needsDaisyBootloader ? (
              <div className="button-sequence two-step">
                <div><span>1</span><i className="hardware-button tap">RESET</i><strong>Tap once</strong></div>
                <ChevronRight />
                <div><span>2</span><i className="hardware-button held">BOOT</i><strong>Press during the pulse</strong></div>
              </div>
            ) : (
              <div className="button-sequence">
                <div><span>1</span><i className="hardware-button held">BOOT</i><strong>Press and hold</strong></div>
                <ChevronRight />
                <div><span>2</span><i className="hardware-button tap">RESET</i><strong>Tap once</strong></div>
                <ChevronRight />
                <div><span>3</span><i className="hardware-button release">BOOT</i><strong>Release</strong></div>
              </div>
            )}
            {incompatibleDfu && (
              <div className="modal-error">
                {needsDaisyBootloader
                  ? "STM32 recovery mode is connected, but this image needs Daisy Bootloader mode. Tap RESET without holding BOOT."
                  : "Daisy Bootloader mode is connected, but this image targets internal flash. Use the BOOT + RESET recovery gesture."}
              </div>
            )}
            <div className="waiting-card">
              <LoaderCircle className="spin" />
              <div><strong>Waiting for Touch 2</strong><small>{needsDaisyBootloader ? "Looking for Daisy Bootloader…" : "Looking for STM32 recovery DFU…"}</small></div>
              <button onClick={onRefreshDevices}><RefreshCw /></button>
            </div>
            {item.managerCompatible && runtime?.serialPort && (
              <button className="text-action" onClick={askForUpdateMode}><Zap /> Ask the instrument to enter update mode automatically</button>
            )}
          </div>
        )}

        {stage === "review" && (
          <div className="review-stage">
            <div className="review-heading"><span className={`review-art tone-${item.tone}`}>{item.name.slice(0, 2).toUpperCase()}</span><div><p className="eyebrow">Ready to install</p><h2>{item.name}</h2><p>{item.author} · {item.version}</p></div></div>
            <div className="safety-grid">
              <div><Cpu /><span><small>Execution</small><strong>{profileNames[item.targetProfile]}</strong></span></div>
              <div><Download /><span><small>Upload address</small><strong>{profileAddresses[item.targetProfile]}</strong></span></div>
              <div><ShieldCheck /><span><small>Integrity</small><strong>{item.checksumMatches === false ? "Mismatch" : "SHA-256 verified"}</strong></span></div>
              <div><Usb /><span><small>Device</small><strong>{devices.find((device) => device.state !== "runtime")?.product ?? "DFU ready"}</strong></span></div>
            </div>
            {item.targetProfile !== "touch2-stm-internal-v1" && <div className="bootloader-note"><PlugZap /><span><strong>Daisy Bootloader required</strong><small>This image is stored at 0x90040000 and cannot be installed through STM32 internal-flash DFU.</small></span></div>}
            {item.targetProfile === "touch2-stm-internal-v1" && <div className="bootloader-note warning"><Activity /><span><strong>Internal-flash instrument</strong><small>This replaces any installed Daisy Bootloader. BOOT + RESET remains available for recovery.</small></span></div>}
            <label className="confirmation-check"><input type="checkbox" checked={confirmed} onChange={(event) => setConfirmed(event.target.checked)} /><span><i><Check /></i>I understand that power and USB must remain connected while the firmware is written.</span></label>
            <button className="install-confirm" disabled={!confirmed || item.checksumMatches === false} onClick={install}><Zap /> Install {item.name}</button>
          </div>
        )}

        {stage === "flashing" && (
          <div className="flashing-stage">
            <div className="progress-orbit"><span /><i><Zap /></i></div>
            <p className="eyebrow">{event.phase.replaceAll("_", " ")}</p>
            <h2>{event.message}</h2>
            <p>Keep the cable connected. Touch Manager will confirm when the instrument returns.</p>
            <div className="phase-track"><i className="done" /><i className={event.phase === "writing" ? "active" : event.phase === "validating" ? "" : "done"} /><i className={event.phase === "awaiting_runtime" ? "active" : ""} /></div>
          </div>
        )}

        {stage === "result" && result && (
          <div className={`result-stage ${result.status}`}>
            <span className="result-icon">{result.status === "succeeded" ? <Check /> : <Activity />}</span>
            <p className="eyebrow">{result.status.replaceAll("_", " ")}</p>
            <h2>{result.status === "succeeded" ? `${item.name} is ready.` : "The transfer needs attention."}</h2>
            <p>{result.message}</p>
            <button className="install-confirm" onClick={onClose}>Done</button>
            <details><summary>Technical details</summary><pre>{result.transcript}</pre></details>
          </div>
        )}
      </section>
    </div>
  );
}

export default App;
