import * as api from "../lib/api";
import { useAppStore } from "../store/appStore";

const STATUS_LABEL: Record<api.TransferStatus, string> = {
  connecting: "Connecting",
  awaitingApproval: "Waiting for the other device",
  active: "Transferring",
  completed: "Done",
  declined: "Declined",
  cancelled: "Cancelled",
  failed: "Failed",
};

export function TransferList() {
  const transfers = useAppStore((state) => state.transfers);
  const setTransfers = useAppStore((state) => state.setTransfers);
  const setError = useAppStore((state) => state.setError);

  const hasFinished = transfers.some((transfer) =>
    ["completed", "declined", "cancelled", "failed"].includes(transfer.status),
  );

  return (
    <div className="panel panel--grow">
      <div className="panel__head">
        <h2>Transfers</h2>
        {hasFinished && (
          <button
            type="button"
            className="link-button"
            onClick={async () => {
              try {
                await api.clearFinishedTransfers();
                setTransfers(await api.listTransfers());
              } catch (err) {
                setError(String(err));
              }
            }}
          >
            Clear finished
          </button>
        )}
      </div>

      {transfers.length === 0 ? (
        <p className="muted empty">Nothing yet.</p>
      ) : (
        <ul className="transfers">
          {transfers.map((transfer) => (
            <TransferRow key={transfer.transferId} transfer={transfer} />
          ))}
        </ul>
      )}
    </div>
  );
}

function TransferRow({ transfer }: { transfer: api.TransferProgress }) {
  const setError = useAppStore((state) => state.setError);

  const percent =
    transfer.totalBytes > 0
      ? Math.min(100, (transfer.transferredBytes / transfer.totalBytes) * 100)
      : transfer.status === "completed"
        ? 100
        : 0;

  const inFlight = ["connecting", "awaitingApproval", "active"].includes(transfer.status);

  return (
    <li className={`transfer transfer--${transfer.status}`}>
      <div className="transfer__top">
        <span className="transfer__arrow" aria-hidden="true">
          {transfer.direction === "send" ? "↑" : "↓"}
        </span>
        <span className="transfer__peer">{transfer.peerName || "Unknown device"}</span>
        <span className="transfer__status">{STATUS_LABEL[transfer.status]}</span>
      </div>

      <div className="progress" role="progressbar" aria-valuenow={Math.round(percent)}>
        <div className="progress__bar" style={{ width: `${percent}%` }} />
      </div>

      <div className="transfer__bottom">
        <span className="muted small">
          {transfer.filesCompleted}/{transfer.fileCount} files ·{" "}
          {api.formatBytes(transfer.transferredBytes)} of{" "}
          {api.formatBytes(transfer.totalBytes)}
          {transfer.status === "active" && transfer.bytesPerSecond > 0 && (
            <> · {api.formatRate(transfer.bytesPerSecond)}</>
          )}
          {api.formatEta(transfer) && <> · {api.formatEta(transfer)}</>}
        </span>

        {inFlight && (
          <button
            type="button"
            className="link-button"
            onClick={() => api.cancelTransfer(transfer.transferId).catch((err) => setError(String(err)))}
          >
            Cancel
          </button>
        )}
      </div>

      {transfer.currentFile && transfer.status === "active" && (
        <p className="transfer__file muted small" title={transfer.currentFile}>
          {transfer.currentFile}
        </p>
      )}

      {transfer.error && <p className="transfer__error small">{transfer.error}</p>}
    </li>
  );
}
