import * as api from "../lib/api";
import { useAppStore } from "../store/appStore";

/// A device we have paired with presented a different key.
///
/// Shown as a persistent banner rather than a dismissible toast, and with no
/// "trust anyway" button. There are two explanations — the peer reinstalled,
/// or someone is impersonating it — and the app cannot tell them apart. The
/// only safe resolution is for the user to forget the device deliberately and
/// pair again by comparing codes, which is what the button here does.
export function IdentityAlert({ warning }: { warning: api.IdentityWarning }) {
  const dismiss = useAppStore((state) => state.dismissIdentityWarning);
  const refreshTrusted = useAppStore((state) => state.refreshTrusted);
  const setError = useAppStore((state) => state.setError);

  async function forget() {
    try {
      await api.forgetPeer(warning.peerDeviceId);
      await refreshTrusted();
      dismiss(warning.peerDeviceId);
    } catch (err) {
      setError(String(err));
    }
  }

  return (
    <div className="banner banner--danger" role="alert">
      <div>
        <strong>{warning.peerName} presented a different identity key.</strong>
        <p className="small">
          The connection was refused. This happens if that device reinstalled fluqsr — or
          if something is impersonating it. Only re-pair if you are expecting the change,
          and compare the codes on both screens when you do.
        </p>
        <details className="details">
          <summary>Compare fingerprints</summary>
          <div className="fingerprint-compare">
            <div>
              <span className="muted small">Expected</span>
              <code className="fingerprint fingerprint--full">
                {api.groupFingerprint(warning.expectedFingerprint)}
              </code>
            </div>
            <div>
              <span className="muted small">Presented</span>
              <code className="fingerprint fingerprint--full">
                {api.groupFingerprint(warning.presentedFingerprint)}
              </code>
            </div>
          </div>
        </details>
      </div>

      <div className="banner__actions">
        <button type="button" onClick={forget}>
          Forget device
        </button>
        <button
          type="button"
          className="ghost"
          onClick={() => dismiss(warning.peerDeviceId)}
        >
          Dismiss
        </button>
      </div>
    </div>
  );
}
