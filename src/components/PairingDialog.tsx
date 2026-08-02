import * as api from "../lib/api";
import { useAppStore } from "../store/appStore";

/// First contact with a device. Both screens show the same six digits, and the
/// user confirms they match.
///
/// This is the only thing standing between the app and a man-in-the-middle, so
/// the wording deliberately asks the user to *compare* rather than to approve.
/// A dialog that reads like a routine permission prompt gets clicked through;
/// one that asks a question with a checkable answer does not.
export function PairingDialog({ request }: { request: api.PairingRequest }) {
  const resolvePairing = useAppStore((state) => state.resolvePairing);

  return (
    <div className="modal-backdrop">
      <div className="modal" role="dialog" aria-modal="true">
        <h2>Pair with {request.peerName}?</h2>
        <p className="muted">
          {request.direction === "send"
            ? "You have not sent to this device before."
            : "This device has not connected to you before."}
        </p>

        <div className="code" aria-label="Pairing code">
          {request.pairingCode.split("").map((digit, index) => (
            <span key={index} className="code__digit">
              {digit}
            </span>
          ))}
        </div>

        <p className="pairing__instruction">
          Check that <strong>{request.peerName}</strong> is showing these same six digits.
        </p>
        <p className="muted small">
          If the numbers differ, someone may be intercepting the connection. Do not pair.
        </p>

        <details className="details">
          <summary>Device fingerprint</summary>
          <code className="fingerprint fingerprint--full">
            {api.groupFingerprint(request.peerFingerprint)}
          </code>
        </details>

        <div className="modal__actions">
          <button
            type="button"
            className="ghost"
            onClick={() => void resolvePairing(request.requestId, false)}
          >
            They don't match
          </button>
          <button
            type="button"
            className="primary"
            onClick={() => void resolvePairing(request.requestId, true)}
          >
            The codes match
          </button>
        </div>
      </div>
    </div>
  );
}
