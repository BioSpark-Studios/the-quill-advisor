import { useEffect, useState } from "react";
import { api, type BillingEntry, type BillingLedger as Ledger } from "../../lib/ipc";
import type { PluginProps } from "../../plugins/registry";

const TODAY = new Date().toISOString().slice(0, 10);

function fmtHours(minutes: number): string {
  const sign = minutes < 0 ? "-" : "";
  const abs = Math.abs(minutes);
  const h = Math.floor(abs / 60);
  const m = abs % 60;
  if (h === 0) return `${sign}${m}m`;
  if (m === 0) return `${sign}${h}h`;
  return `${sign}${h}h ${m}m`;
}

function fmtDate(iso: string): string {
  const d = new Date(`${iso}T00:00:00`);
  if (Number.isNaN(d.getTime())) return iso;
  return d.toLocaleDateString(undefined, { month: "short", day: "numeric", year: "numeric" });
}

/**
 * Billing & Hours — a vault-level ledger (not per-chamber): the family's
 * purchased retainer, time logged against it, and the running balance.
 */
export function BillingLedger({ vault, onClose }: PluginProps) {
  const [ledger, setLedger] = useState<Ledger | null>(null);
  const [hours, setHours] = useState("1");
  const [description, setDescription] = useState("");
  const [billedAt, setBilledAt] = useState(TODAY);
  const [topUp, setTopUp] = useState("");
  const [busy, setBusy] = useState(false);

  async function refresh() {
    setLedger(await api.getBillingLedger(vault.id));
  }

  useEffect(() => {
    refresh();
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [vault.id]);

  async function logEntry() {
    const parsed = Number(hours);
    if (!description.trim() || !Number.isFinite(parsed) || parsed <= 0 || busy) return;
    setBusy(true);
    try {
      await api.logBillingEntry(vault.id, null, Math.round(parsed * 60), description.trim(), billedAt);
      setDescription("");
      setHours("1");
      await refresh();
    } finally {
      setBusy(false);
    }
  }

  async function addRetainer() {
    const parsed = Number(topUp);
    if (!ledger || !Number.isFinite(parsed) || parsed <= 0 || busy) return;
    setBusy(true);
    try {
      await api.setRetainerMinutes(vault.id, ledger.retainerMinutes + Math.round(parsed * 60));
      setTopUp("");
      await refresh();
    } finally {
      setBusy(false);
    }
  }

  async function remove(entry: BillingEntry) {
    setLedger((prev) =>
      prev
        ? {
            ...prev,
            entries: prev.entries.filter((e) => e.id !== entry.id),
            usedMinutes: prev.usedMinutes - entry.minutes,
            balanceMinutes: prev.balanceMinutes + entry.minutes,
          }
        : prev,
    );
    await api.deleteBillingEntry(vault.id, entry.id);
  }

  const balance = ledger?.balanceMinutes ?? 0;
  const low = ledger !== null && ledger.retainerMinutes > 0 && balance <= ledger.retainerMinutes * 0.15;

  return (
    <div className="fixed inset-0 z-40 flex items-center justify-center bg-black/50 p-4" onClick={onClose}>
      <div
        onClick={(e) => e.stopPropagation()}
        className="flex h-[640px] w-full max-w-2xl flex-col overflow-hidden rounded-2xl border border-border bg-surface-raised shadow-2xl"
      >
        <div className="flex items-center justify-between border-b border-border px-5 py-3">
          <div>
            <h2 className="font-serif text-lg text-ink">Billing &amp; Hours</h2>
            <p className="text-xs text-ink-muted">{vault.name}</p>
          </div>
          <button onClick={onClose} className="text-ink-muted hover:text-ink" aria-label="Close">
            ✕
          </button>
        </div>

        {ledger && (
          <div className="grid grid-cols-3 gap-2 border-b border-border p-4">
            <div className="rounded-lg bg-primary/5 py-2 text-center">
              <div className="text-lg font-semibold text-primary">{fmtHours(ledger.retainerMinutes)}</div>
              <div className="text-[10px] uppercase tracking-wide text-ink-muted">Retainer</div>
            </div>
            <div className="rounded-lg bg-primary/5 py-2 text-center">
              <div className="text-lg font-semibold text-ink">{fmtHours(ledger.usedMinutes)}</div>
              <div className="text-[10px] uppercase tracking-wide text-ink-muted">Used</div>
            </div>
            <div className={`rounded-lg py-2 text-center ${low ? "bg-red-500/10" : "bg-primary/5"}`}>
              <div className={`text-lg font-semibold ${low ? "text-red-500" : "text-primary"}`}>
                {fmtHours(balance)}
              </div>
              <div className="text-[10px] uppercase tracking-wide text-ink-muted">Balance</div>
            </div>
          </div>
        )}

        {/* Top up the retainer */}
        <form
          onSubmit={(e) => {
            e.preventDefault();
            addRetainer();
          }}
          className="flex items-center gap-2 border-b border-border px-4 py-2"
        >
          <span className="text-xs text-ink-muted">Add hours to retainer:</span>
          <input
            type="number"
            min="0"
            step="0.5"
            value={topUp}
            onChange={(e) => setTopUp(e.target.value)}
            placeholder="e.g. 10"
            className="w-20 rounded-lg border border-border bg-surface px-2 py-1 text-sm text-ink outline-none focus:border-primary"
          />
          <button
            type="submit"
            disabled={busy}
            className="rounded-lg border border-border px-3 py-1 text-xs font-medium text-ink hover:bg-primary/5 disabled:opacity-50"
          >
            Top up
          </button>
        </form>

        {/* Log time worked */}
        <form
          onSubmit={(e) => {
            e.preventDefault();
            logEntry();
          }}
          className="flex flex-wrap gap-2 border-b border-border p-3"
        >
          <input
            type="number"
            min="0"
            step="0.25"
            value={hours}
            onChange={(e) => setHours(e.target.value)}
            className="w-20 rounded-lg border border-border bg-surface px-3 py-1.5 text-sm text-ink outline-none focus:border-primary"
            aria-label="Hours"
          />
          <input
            value={description}
            onChange={(e) => setDescription(e.target.value)}
            placeholder="What was the time for? (e.g. 'Essay review call')"
            className="min-w-[160px] flex-1 rounded-lg border border-border bg-surface px-3 py-1.5 text-sm text-ink outline-none focus:border-primary"
          />
          <input
            type="date"
            value={billedAt}
            onChange={(e) => setBilledAt(e.target.value)}
            className="rounded-lg border border-border bg-surface px-3 py-1.5 text-sm text-ink outline-none focus:border-primary"
          />
          <button
            type="submit"
            disabled={busy}
            className="rounded-lg bg-primary px-4 py-1.5 text-sm font-medium text-surface-raised disabled:opacity-50"
          >
            Log time
          </button>
        </form>

        {/* Entry history */}
        <div className="flex-1 overflow-y-auto p-5">
          {!ledger || ledger.entries.length === 0 ? (
            <p className="text-sm text-ink-muted">No time logged yet — log the first session above.</p>
          ) : (
            <ul className="space-y-2">
              {ledger.entries.map((entry) => (
                <li
                  key={entry.id}
                  className="flex items-center justify-between rounded-lg border border-border bg-surface px-3 py-2"
                >
                  <div>
                    <div className="text-sm text-ink">{entry.description}</div>
                    <div className="text-xs text-ink-muted">{fmtDate(entry.billedAt)}</div>
                  </div>
                  <div className="flex items-center gap-3">
                    <span className="text-sm font-medium text-ink">{fmtHours(entry.minutes)}</span>
                    <button
                      onClick={() => remove(entry)}
                      className="text-ink-muted hover:text-red-500"
                      aria-label="Delete entry"
                      title="Delete entry"
                    >
                      ✕
                    </button>
                  </div>
                </li>
              ))}
            </ul>
          )}
        </div>
      </div>
    </div>
  );
}
