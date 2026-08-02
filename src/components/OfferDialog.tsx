import * as api from "../lib/api";
import { useAppStore } from "../store/appStore";

/// Consent gate for an incoming transfer. Shows who is sending, how much, and
/// what — deciding to accept should not require guessing at any of those.
export function OfferDialog({ offer }: { offer: api.IncomingOffer }) {
  const resolveOffer = useAppStore((state) => state.resolveOffer);
  const trusted = useAppStore((state) => state.trusted);
  const receiveDir = useAppStore((state) => state.self?.settings.receiveDir);

  const isPaired = trusted.some((peer) => peer.deviceId === offer.peerDeviceId);
  const remaining = offer.fileCount - offer.preview.length;

  return (
    <div className="modal-backdrop">
      <div className="modal" role="dialog" aria-modal="true">
        <h2>{offer.peerName} wants to send you files</h2>

        <p className="offer__summary">
          <strong>{offer.fileCount}</strong> file{offer.fileCount === 1 ? "" : "s"} ·{" "}
          <strong>{api.formatBytes(offer.totalBytes)}</strong>
        </p>

        <ul className="offer__files">
          {offer.preview.map((path) => (
            <li key={path} title={path}>
              {path}
            </li>
          ))}
          {remaining > 0 && <li className="muted">and {remaining} more…</li>}
        </ul>

        {receiveDir && (
          <p className="muted small">
            Saving to <code>{receiveDir}</code>
          </p>
        )}

        {!isPaired && (
          <p className="muted small">This device is not in your paired list.</p>
        )}

        <details className="details">
          <summary>Device fingerprint</summary>
          <code className="fingerprint fingerprint--full">
            {api.groupFingerprint(offer.peerFingerprint)}
          </code>
        </details>

        <div className="modal__actions">
          <button
            type="button"
            className="ghost"
            onClick={() => void resolveOffer(offer.transferId, false)}
          >
            Decline
          </button>
          <button
            type="button"
            className="primary"
            onClick={() => void resolveOffer(offer.transferId, true)}
          >
            Accept
          </button>
        </div>
      </div>
    </div>
  );
}
