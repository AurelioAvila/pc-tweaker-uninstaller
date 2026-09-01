import { text } from "../i18n";
import type { FilterChip, FlowState, ProgramInfo, SortKey } from "../types";
import {
  badgeHint,
  badgeLabel,
  batchOrder,
  confidenceLabel,
  confidenceTitle,
  formatSize,
  hueOf,
  isBatchable,
  isFamilyApp,
  monogram,
  sourceLabel,
} from "../utils";

interface ProgramListProps {
  query: string;
  setQuery: React.Dispatch<React.SetStateAction<string>>;
  chip: FilterChip;
  setChip: React.Dispatch<React.SetStateAction<FilterChip>>;
  showHidden: boolean;
  setShowHidden: React.Dispatch<React.SetStateAction<boolean>>;
  filtered: ProgramInfo[];
  visible: ProgramInfo[];
  totalSizeKb: number;
  batchSelected: readonly string[];
  setBatchSelected: React.Dispatch<React.SetStateAction<readonly string[]>>;
  flow: FlowState;
  setFlow: React.Dispatch<React.SetStateAction<FlowState>>;
  programs: ProgramInfo[];
  sortKey: SortKey;
  sortAsc: boolean;
  toggleSort: (key: SortKey) => void;
  expandedKey: string | null;
  toggleExpanded: (key: string) => void;
  beginUninstall: (program: ProgramInfo) => void;
  openFolder: (program: ProgramInfo) => void;
  detailNotice: string | null;
}

export function ProgramList({
  query,
  setQuery,
  chip,
  setChip,
  showHidden,
  setShowHidden,
  filtered,
  visible,
  totalSizeKb,
  batchSelected,
  setBatchSelected,
  flow,
  setFlow,
  programs,
  sortKey,
  sortAsc,
  toggleSort,
  expandedKey,
  toggleExpanded,
  beginUninstall,
  openFolder,
  detailNotice,
}: ProgramListProps) {
  return (
    <>
      <div className="toolbar">
        <input
          type="search"
          className="search"
          placeholder={text.programs.searchPlaceholder}
          aria-label={text.programs.searchLabel}
          value={query}
          onChange={(e) => {
            setQuery(e.target.value);
          }}
        />
        <div className="chip-group" role="group" aria-label={text.programs.filterAll}>
          {(
            [
              ["all", text.programs.filterAll],
              ["large", text.programs.filterLarge],
              ["recent", text.programs.filterRecent],
            ] as const
          ).map(([value, label]) => (
            <button
              key={value}
              type="button"
              className={`chip chip-button${chip === value ? " chip-active" : ""}`}
              aria-pressed={chip === value}
              onClick={() => {
                setChip(value);
              }}
            >
              {label}
            </button>
          ))}
          <button
            type="button"
            className={`chip chip-button${showHidden ? " chip-active" : ""}`}
            aria-pressed={showHidden}
            title={text.programs.badgeHiddenHint}
            onClick={() => {
              setShowHidden((v) => !v);
            }}
          >
            {text.programs.showHidden}
          </button>
        </div>
        <span className="chip" role="status">
          {text.programs.countSummary(filtered.length, visible.length)}
        </span>
        {totalSizeKb > 0 && (
          <span className="chip chip-accent">
            {formatSize(totalSizeKb)} {text.programs.statTotalSize}
          </span>
        )}
      </div>

      {programs.length === 0 && (
        <section className="empty">
          <h2>{text.programs.emptyTitle}</h2>
          <p>{text.programs.emptyBody}</p>
        </section>
      )}

      {programs.length > 0 && filtered.length === 0 && (
        <section className="empty">
          <h2>{text.programs.noMatchesTitle}</h2>
          <p>{text.programs.noMatchesBody}</p>
        </section>
      )}

      {filtered.length > 0 && (
        <>
          {batchSelected.length > 0 && (
            <div className="batch-bar">
              <button
                type="button"
                className="button primary"
                disabled={flow.step !== "idle"}
                onClick={() => {
                  const chosen = batchOrder(
                    programs.filter((p) => batchSelected.includes(`${p.source}:${p.id}`)),
                  );
                  if (chosen.length > 0) setFlow({ step: "batchConfirm", programs: chosen });
                }}
              >
                {text.uninstall.batchBar(
                  batchSelected.length,
                  formatSize(
                    programs
                      .filter((p) => batchSelected.includes(`${p.source}:${p.id}`))
                      .reduce((sum, p) => sum + (p.estimatedSizeKb ?? 0), 0),
                  ),
                )}
              </button>
              <button
                type="button"
                className="button"
                onClick={() => {
                  setBatchSelected([]);
                }}
              >
                {text.uninstall.batchClear}
              </button>
            </div>
          )}
          <div className="list" role="table" aria-label={text.programs.columnProgram}>
            <div className="list-head" role="row">
              <span
                role="columnheader"
                aria-sort={sortKey === "name" ? (sortAsc ? "ascending" : "descending") : "none"}
              >
                <button
                  type="button"
                  className="sort-button"
                  onClick={() => {
                    toggleSort("name");
                  }}
                >
                  {text.programs.columnProgram}
                  {sortKey === "name" && <span aria-hidden="true">{sortAsc ? "▲" : "▼"}</span>}
                </button>
              </span>
              <span role="columnheader">{text.programs.columnVersion}</span>
              <span
                role="columnheader"
                className="num"
                aria-sort={sortKey === "size" ? (sortAsc ? "ascending" : "descending") : "none"}
              >
                <button
                  type="button"
                  className="sort-button"
                  onClick={() => {
                    toggleSort("size");
                  }}
                >
                  {text.programs.columnSize}
                  {sortKey === "size" && <span aria-hidden="true">{sortAsc ? "▲" : "▼"}</span>}
                </button>
              </span>
              <span
                role="columnheader"
                aria-sort={sortKey === "date" ? (sortAsc ? "ascending" : "descending") : "none"}
              >
                <button
                  type="button"
                  className="sort-button"
                  onClick={() => {
                    toggleSort("date");
                  }}
                >
                  {text.programs.columnInstalled}
                  {sortKey === "date" && <span aria-hidden="true">{sortAsc ? "▲" : "▼"}</span>}
                </button>
              </span>
              <span role="columnheader" className="num" />
              <span role="columnheader" className="num" />
            </div>
            {filtered.map((p) => {
              const rowKey = `${p.source}:${p.id}`;
              const expanded = expandedKey === rowKey;
              return (
                <div key={rowKey} className={expanded ? "row-group expanded" : "row-group"}>
                  <div
                    className="row"
                    role="row"
                    aria-expanded={expanded}
                    onClick={() => {
                      toggleExpanded(rowKey);
                    }}
                  >
                    <span role="cell" className="cell-main">
                      <input
                        type="checkbox"
                        className="batch-check"
                        disabled={!isBatchable(p)}
                        title={isBatchable(p) ? undefined : text.uninstall.batchNotBatchable}
                        checked={batchSelected.includes(rowKey)}
                        onClick={(e) => {
                          e.stopPropagation();
                        }}
                        onChange={() => {
                          setBatchSelected((current) =>
                            current.includes(rowKey)
                              ? current.filter((k) => k !== rowKey)
                              : [...current, rowKey],
                          );
                        }}
                      />
                      <span
                        className="avatar"
                        aria-hidden="true"
                        style={{ background: `hsl(${String(hueOf(p.name))} 45% 26%)` }}
                      >
                        {monogram(p.name)}
                      </span>
                      <span className="titles">
                        <span className="name">{p.name}</span>
                        <span className="publisher">{p.publisher ?? " "}</span>
                      </span>
                    </span>
                    <span role="cell" className="dim">
                      {p.version ?? "—"}
                    </span>
                    <span role="cell" className="dim num">
                      {formatSize(p.estimatedSizeKb)}
                    </span>
                    <span role="cell" className="dim">
                      {p.installDate ?? "—"}
                    </span>
                    <span role="cell" className="cell-badges num">
                      <span
                        className={`conf-chip conf-${p.confidence.level}`}
                        title={confidenceTitle(p.confidence)}
                      >
                        <span className="conf-dot" aria-hidden="true" />
                        {confidenceLabel(p.confidence.level)}
                      </span>
                      {isFamilyApp(p) && (
                        <span className="badge badge-suite" title={text.programs.badgeSuiteHint}>
                          {text.programs.badgeSuite}
                        </span>
                      )}
                      {p.hidden && (
                        <span className="badge badge-hidden" title={text.programs.badgeHiddenHint}>
                          {text.programs.badgeHidden}
                        </span>
                      )}
                      {p.source === "user" && (
                        <span className="badge badge-user" title={text.programs.badgeUserHint}>
                          {text.programs.badgeUser}
                        </span>
                      )}
                      <span className={`badge badge-${p.uninstall}`} title={badgeHint(p.uninstall)}>
                        {badgeLabel(p.uninstall)}
                      </span>
                    </span>
                    <span role="cell" className="cell-action num">
                      {(p.uninstall === "msi" ||
                        p.uninstall === "executable" ||
                        p.uninstall === "store") && (
                        <button
                          type="button"
                          className="row-action"
                          disabled={flow.step !== "idle"}
                          onClick={(e) => {
                            e.stopPropagation();
                            beginUninstall(p);
                          }}
                        >
                          {text.uninstall.action}
                        </button>
                      )}
                    </span>
                  </div>
                  {expanded && (
                    <div className="row-details" role="row">
                      <div role="cell" className="details-grid">
                        <div>
                          <span className="detail-label">{text.programs.detailSource}</span>
                          <span className="detail-value">{sourceLabel(p.source)}</span>
                        </div>
                        <div>
                          <span className="detail-label">
                            {/* An MSIX package has no registry entry to
                                    name; calling its package name one would
                                    be a small lie in a panel whose whole job
                                    is to show exactly what this app read. */}
                            {p.source === "store"
                              ? text.programs.detailPackageName
                              : text.programs.detailKey}
                          </span>
                          <span className="detail-value mono">{p.id}</span>
                        </div>
                        <div className="detail-wide">
                          <span className="detail-label">{text.programs.detailLocation}</span>
                          <span className="detail-value mono">
                            {p.installLocation ?? text.programs.detailNoLocation}
                          </span>
                        </div>
                        {/* Why this confidence band: the evidence itself,
                                one line per reason, plus the honesty note. */}
                        <div className="detail-wide">
                          <span className="detail-label">{text.uninstall.confidenceLabel}</span>
                          <span className={`detail-value conf-text-${p.confidence.level}`}>
                            {confidenceLabel(p.confidence.level)}
                          </span>
                          <ul className="conf-reasons">
                            {p.confidence.reasons.map((r) => (
                              <li key={r}>{text.confidence.reasons[r]}</li>
                            ))}
                            <li className="conf-disclaimer">{text.confidence.disclaimer}</li>
                          </ul>
                        </div>
                        <div className="detail-actions">
                          {p.installLocation !== null && (
                            <button
                              type="button"
                              className="button-ghost small"
                              onClick={() => {
                                openFolder(p);
                              }}
                            >
                              {text.programs.openFolder}
                            </button>
                          )}
                        </div>
                        {detailNotice !== null && (
                          <p className="detail-notice" role="alert">
                            {detailNotice}
                          </p>
                        )}
                      </div>
                    </div>
                  )}
                </div>
              );
            })}
          </div>
        </>
      )}
    </>
  );
}
