import { text } from "../i18n";
import type { FlowState, ProgramInfo } from "../types";
import {
  confidenceLabel,
  displayCommand,
  formatSize,
  isFamilyApp,
  restorePointLine,
} from "../utils";
import type { UninstallerEntitlement, AccountState } from "../account";

interface UninstallFlowDialogsProps {
  flow: FlowState;
  closeFlow: () => void;
  confirmUninstall: (program: ProgramInfo) => void;
  confirmStoreRemoval: (program: ProgramInfo) => void;
  beginResidueScan: (program: ProgramInfo) => void;
  runResidueClean: (program: ProgramInfo, selected: readonly string[]) => void;
  setFlow: React.Dispatch<React.SetStateAction<FlowState>>;
  uninstallerPro: UninstallerEntitlement | null;
  account: AccountState;
  checkoutBusy: boolean;
  beginProCheckout: () => void;
  runBatch: (programs: ProgramInfo[]) => Promise<void>;
}

export function UninstallFlowDialogs({
  flow,
  closeFlow,
  confirmUninstall,
  confirmStoreRemoval,
  beginResidueScan,
  runResidueClean,
  setFlow,
  uninstallerPro,
  account,
  checkoutBusy,
  beginProCheckout,
  runBatch,
}: UninstallFlowDialogsProps) {
  if (flow.step === "idle") return null;

  return (
    <div
      className="overlay"
      role="presentation"
      onClick={
        flow.step === "planning" || flow.step === "running" || flow.step === "storeRunning"
          ? undefined
          : closeFlow
      }
    >
      <div
        className="dialog"
        role="dialog"
        aria-modal="true"
        onClick={(e) => {
          e.stopPropagation();
        }}
      >
        {flow.step === "planning" && (
          <p className="status" role="status">
            <span className="spinner" aria-hidden="true" />
            {text.uninstall.planning}
          </p>
        )}

        {flow.step === "planError" && (
          <>
            <h2>{text.uninstall.planFailedTitle}</h2>
            <p className="dialog-body">{flow.message}</p>
            <div className="dialog-actions">
              <button type="button" className="button" onClick={closeFlow}>
                {text.uninstall.close}
              </button>
            </div>
          </>
        )}

        {flow.step === "confirm" && (
          <>
            <h2>{text.uninstall.confirmTitle(flow.plan.programName)}</h2>
            <p className="dialog-body">{text.uninstall.confirmBody}</p>

            {/* Relationship Map: evidence-based warnings before anything
                runs. Dependents = programs installed inside this one's
                folder; removing it takes their files too. */}
            {flow.program.relations.dependents.length > 0 && (
              <p className="dialog-body reboot">
                {text.uninstall.relDependentsWarning(
                  flow.program.relations.dependents.slice(0, 6).join(", ") +
                    (flow.program.relations.dependents.length > 6 ? "…" : ""),
                )}
              </p>
            )}
            {flow.program.relations.installedVia !== null && (
              <p className="dialog-body subtle">
                {text.uninstall.relInstalledVia(flow.program.relations.installedVia)}
              </p>
            )}
            {flow.program.relations.publisherSiblings > 0 && (
              <p className="dialog-body subtle">
                {text.uninstall.relSiblings(flow.program.relations.publisherSiblings)}
              </p>
            )}

            {/* The Removal Brief: what will happen, in scannable rows,
                before any button is pressed. */}
            <div className="brief-grid">
              <span className="brief-label">{text.uninstall.methodLabel}</span>
              <span className="brief-value">
                {flow.plan.kind === "msi" ? text.uninstall.methodMsi : text.uninstall.methodExe}
              </span>
              <span className="brief-label">{text.uninstall.privilegesLabel}</span>
              <span className="brief-value">
                {flow.plan.needsElevation
                  ? text.uninstall.privilegesAdmin
                  : text.uninstall.privilegesUser}
              </span>
              <span className="brief-label">{text.uninstall.sizeLabel}</span>
              <span className="brief-value">
                {flow.plan.estimatedSizeKb !== null && flow.plan.estimatedSizeKb > 0
                  ? formatSize(flow.plan.estimatedSizeKb)
                  : text.uninstall.sizeUnknown}
              </span>
              <span className="brief-label">{text.uninstall.confidenceLabel}</span>
              <span className={`brief-value conf-text-${flow.plan.confidence.level}`}>
                {confidenceLabel(flow.plan.confidence.level)}
                <span className="brief-sub">
                  {flow.plan.confidence.reasons.map((r) => text.confidence.reasons[r]).join(" ")}
                </span>
              </span>
            </div>

            <p className="command-label">{text.uninstall.commandLabel}</p>
            <code className="command">{displayCommand(flow.plan.command)}</code>
            <ul className="dialog-notes">
              {flow.plan.needsElevation && <li>{text.uninstall.elevationNote}</li>}
              {flow.plan.willAttemptRestorePoint && <li>{text.uninstall.restorePointNote}</li>}
              {flow.plan.warnings.map((warning) => (
                <li key={warning}>{warning}</li>
              ))}
              {isFamilyApp(flow.program) && (
                <li className="note-suite">{text.uninstall.familyNote}</li>
              )}
              {flow.program.hidden && <li className="note-warn">{text.uninstall.hiddenNote}</li>}
              <li>{text.uninstall.notRemovedNote}</li>
            </ul>
            <div className="dialog-actions">
              <button type="button" className="button-ghost" onClick={closeFlow}>
                {text.uninstall.cancel}
              </button>
              <button
                type="button"
                className="button button-danger"
                onClick={() => {
                  confirmUninstall(flow.program);
                }}
              >
                {text.uninstall.confirm}
              </button>
            </div>
          </>
        )}

        {flow.step === "storeConfirm" && (
          <>
            <h2>{text.uninstall.confirmTitle(flow.program.name)}</h2>
            <p className="dialog-body">{text.uninstall.storeConfirmBody}</p>
            <div className="dialog-actions">
              <button type="button" className="button" onClick={closeFlow}>
                {text.uninstall.cancel}
              </button>
              <button
                type="button"
                className="button button-danger"
                onClick={() => {
                  confirmStoreRemoval(flow.program);
                }}
              >
                {text.uninstall.confirm}
              </button>
            </div>
          </>
        )}

        {flow.step === "storeRunning" && (
          <p className="status" role="status">
            <span className="spinner" aria-hidden="true" />
            {text.uninstall.storeRemoving}
          </p>
        )}

        {flow.step === "storeDone" && (
          <>
            <h2>{text.uninstall.reportSuccessTitle}</h2>
            <p className="dialog-body">{text.uninstall.storeRemoved(flow.program.name)}</p>
            <div className="dialog-actions">
              <button type="button" className="button" onClick={closeFlow}>
                {text.uninstall.close}
              </button>
            </div>
          </>
        )}

        {flow.step === "running" && (
          <>
            <p className="status" role="status">
              <span className="spinner" aria-hidden="true" />
              {text.uninstall.running(flow.program.name)}
            </p>
            <p className="dialog-body">{text.uninstall.runningNote}</p>
          </>
        )}

        {flow.step === "report" && (
          <>
            <h2>
              {flow.report.success
                ? text.uninstall.reportSuccessTitle
                : text.uninstall.reportFailureTitle}
            </h2>
            <p className="dialog-body">{flow.report.message}</p>
            {flow.report.rebootRequired && (
              <p className="dialog-body reboot">{text.uninstall.rebootNote}</p>
            )}
            <ul className="dialog-notes">
              <li>{restorePointLine(flow.report.restorePoint)}</li>
              {flow.report.exitCode !== null && (
                <li>
                  {text.uninstall.exitCodeLabel}: {String(flow.report.exitCode)}
                </li>
              )}
            </ul>
            <div className="dialog-actions">
              {flow.report.success && (
                <button
                  type="button"
                  className="button primary"
                  onClick={() => {
                    beginResidueScan(flow.program);
                  }}
                >
                  {text.uninstall.residueScan}
                </button>
              )}
              <button type="button" className="button" onClick={closeFlow}>
                {text.uninstall.close}
              </button>
            </div>
          </>
        )}

        {flow.step === "residueScanning" && (
          <>
            <h2>{text.uninstall.residueScanning}</h2>
            <p className="dialog-body">{flow.program.name}</p>
          </>
        )}

        {flow.step === "residue" && (
          <>
            <h2>
              {flow.residue.items.length === 0
                ? text.uninstall.reportSuccessTitle
                : text.uninstall.residueTitle}
            </h2>
            {flow.residue.items.length === 0 ? (
              <p className="dialog-body">{text.uninstall.residueNone}</p>
            ) : (
              <>
                <p className="dialog-body">
                  {text.uninstall.residueIntro(
                    flow.residue.items.length,
                    (flow.residue.totalKb / 1024).toFixed(1),
                  )}
                </p>
                <ul className="residue-list">
                  {flow.residue.items.map((item) => (
                    <li key={item.path} className="residue-item">
                      <label>
                        <input
                          type="checkbox"
                          disabled={!item.deletable}
                          checked={flow.selected.includes(item.path)}
                          onChange={() => {
                            setFlow({
                              ...flow,
                              selected: flow.selected.includes(item.path)
                                ? flow.selected.filter((p) => p !== item.path)
                                : [...flow.selected, item.path],
                            });
                          }}
                        />
                        <span className="residue-kind">
                          {text.uninstall.residueKinds[item.kind] ?? item.kind}
                        </span>
                        <span className="residue-path">{item.path}</span>
                        {item.sizeKb !== null && (
                          <span className="residue-size">{formatSize(item.sizeKb)}</span>
                        )}
                      </label>
                    </li>
                  ))}
                </ul>
                <p className="dialog-body subtle">{text.uninstall.residueRegistryNote}</p>
              </>
            )}
            {flow.residue.items.length > 0 && !uninstallerPro?.active && (
              <p className="dialog-body reboot">
                {text.menu.proGateResidue}
                {account.status !== "signed-in" && ` ${text.menu.proGateSignIn}`}
              </p>
            )}
            <div className="dialog-actions">
              {flow.residue.items.length > 0 && uninstallerPro?.active && (
                <button
                  type="button"
                  className="button primary"
                  disabled={flow.selected.length === 0}
                  onClick={() => {
                    runResidueClean(flow.program, flow.selected);
                  }}
                >
                  {text.uninstall.residueClean}
                </button>
              )}
              {flow.residue.items.length > 0 &&
                !uninstallerPro?.active &&
                account.status === "signed-in" && (
                  <button
                    type="button"
                    className="button primary"
                    disabled={checkoutBusy}
                    onClick={beginProCheckout}
                  >
                    {account.isPro ? text.menu.upsGoProLoyalty : text.menu.upsGoPro}
                  </button>
                )}
              <button type="button" className="button" onClick={closeFlow}>
                {text.uninstall.close}
              </button>
            </div>
          </>
        )}

        {flow.step === "residueDone" && (
          <>
            <h2>{text.uninstall.residueTitle}</h2>
            <p className="dialog-body">
              {text.uninstall.residueDone(
                flow.result.removed.length,
                (flow.result.freedKb / 1024).toFixed(1),
              )}
            </p>
            {flow.result.failed.length > 0 && (
              <p className="dialog-body subtle">
                {text.uninstall.residueFailed(flow.result.failed.length)}
              </p>
            )}
            <div className="dialog-actions">
              <button type="button" className="button" onClick={closeFlow}>
                {text.uninstall.close}
              </button>
            </div>
          </>
        )}

        {flow.step === "batchConfirm" && (
          <>
            <h2>{text.uninstall.batchConfirmTitle(flow.programs.length)}</h2>
            <p className="dialog-body">{text.uninstall.batchConfirmBody}</p>
            <ul className="residue-list">
              {flow.programs.map((p) => (
                <li key={`${p.source}:${p.id}`} className="residue-item">
                  <label>
                    <span className="residue-kind">{p.name}</span>
                    <span className="residue-path">{p.publisher ?? ""}</span>
                    <span className="residue-size">{formatSize(p.estimatedSizeKb)}</span>
                  </label>
                </li>
              ))}
            </ul>
            {!uninstallerPro?.active && (
              <p className="dialog-body reboot">
                {text.menu.proGateBatch}
                {account.status !== "signed-in" && ` ${text.menu.proGateSignIn}`}
              </p>
            )}
            <div className="dialog-actions">
              {uninstallerPro?.active && (
                <button
                  type="button"
                  className="button primary"
                  onClick={() => {
                    void runBatch(flow.programs);
                  }}
                >
                  {text.uninstall.confirm}
                </button>
              )}
              {!uninstallerPro?.active && account.status === "signed-in" && (
                <button
                  type="button"
                  className="button primary"
                  disabled={checkoutBusy}
                  onClick={beginProCheckout}
                >
                  {account.isPro ? text.menu.upsGoProLoyalty : text.menu.upsGoPro}
                </button>
              )}
              <button type="button" className="button" onClick={closeFlow}>
                {text.uninstall.cancel}
              </button>
            </div>
          </>
        )}

        {flow.step === "batchRunning" && (
          <>
            <h2>
              {text.uninstall.batchRunningStep(
                flow.programs[flow.index]?.name ?? "",
                flow.index + 1,
                flow.programs.length,
              )}
            </h2>
            <p className="dialog-body">{text.uninstall.runningNote}</p>
            <ul className="dialog-notes">
              {flow.results.map((r) => (
                <li key={r.name}>
                  {r.success ? "✓" : "✗"} {r.name}
                </li>
              ))}
            </ul>
          </>
        )}

        {flow.step === "batchDone" && (
          <>
            <h2>{text.uninstall.batchDoneTitle}</h2>
            <ul className="dialog-notes">
              {flow.results.map((r) => (
                <li key={r.name}>
                  {r.success ? "✓" : "✗"} {r.name} — {r.message}
                </li>
              ))}
            </ul>
            {flow.results.some((r) => !r.success) && (
              <p className="dialog-body subtle">
                {text.uninstall.batchFailedNote(flow.results.filter((r) => !r.success).length)}
              </p>
            )}
            <div className="dialog-actions">
              <button type="button" className="button" onClick={closeFlow}>
                {text.uninstall.close}
              </button>
            </div>
          </>
        )}

        {flow.step === "execError" && (
          <>
            <h2>{text.uninstall.reportFailureTitle}</h2>
            <p className="dialog-body">{flow.message}</p>
            <div className="dialog-actions">
              <button type="button" className="button" onClick={closeFlow}>
                {text.uninstall.close}
              </button>
            </div>
          </>
        )}
      </div>
    </div>
  );
}
