import { useEffect, useMemo, useState } from "react";
import { api, type PluginRecord } from "../../lib/ipc";
import type { PluginProps } from "../../plugins/registry";

const COLLECTION = "shortlist";
const TIERS = ["Reach", "Target", "Safety"] as const;
type Tier = (typeof TIERS)[number];

const TIER_STYLE: Record<Tier, string> = {
  Reach: "bg-rose-500/15 text-rose-400 border-rose-500/30",
  Target: "bg-amber-500/15 text-amber-500 border-amber-500/30",
  Safety: "bg-emerald-500/15 text-emerald-400 border-emerald-500/30",
};

interface College {
  name: string;
  location: string;
  type: "Private" | "Public" | "LAC";
  admitRate: number; // percent
  tags: string[];
}

/**
 * A small curated atlas. In production this is the front end of a RAG-backed
 * College Landscape Atlas; here it is a fixed, searchable dataset so the browse
 * → shortlist flow is fully exercisable offline.
 */
const COLLEGES: College[] = [
  { name: "Stanford University", location: "Stanford, CA", type: "Private", admitRate: 4, tags: ["CS", "Research", "Entrepreneurship"] },
  { name: "MIT", location: "Cambridge, MA", type: "Private", admitRate: 4, tags: ["Engineering", "STEM", "Research"] },
  { name: "UC Berkeley", location: "Berkeley, CA", type: "Public", admitRate: 11, tags: ["CS", "Public", "Research"] },
  { name: "University of Michigan", location: "Ann Arbor, MI", type: "Public", admitRate: 18, tags: ["Engineering", "Public", "Sports"] },
  { name: "Williams College", location: "Williamstown, MA", type: "LAC", admitRate: 8, tags: ["Liberal Arts", "Small", "Writing"] },
  { name: "Georgia Tech", location: "Atlanta, GA", type: "Public", admitRate: 17, tags: ["Engineering", "CS", "Co-op"] },
  { name: "NYU", location: "New York, NY", type: "Private", admitRate: 12, tags: ["Arts", "Business", "Urban"] },
  { name: "UT Austin", location: "Austin, TX", type: "Public", admitRate: 29, tags: ["Engineering", "Public", "Business"] },
  { name: "Northeastern University", location: "Boston, MA", type: "Private", admitRate: 18, tags: ["Co-op", "Business", "Urban"] },
  { name: "Purdue University", location: "West Lafayette, IN", type: "Public", admitRate: 53, tags: ["Engineering", "Public", "Aviation"] },
  { name: "Arizona State University", location: "Tempe, AZ", type: "Public", admitRate: 88, tags: ["Public", "Business", "Large"] },
  { name: "Kenyon College", location: "Gambier, OH", type: "LAC", admitRate: 34, tags: ["Liberal Arts", "Writing", "Small"] },
];

function suggestTier(admitRate: number): Tier {
  if (admitRate < 15) return "Reach";
  if (admitRate < 40) return "Target";
  return "Safety";
}

/**
 * College Landscape Atlas — browse a searchable college catalog and build a
 * balanced reach/target/safety shortlist stored in the vault's record store.
 */
export function CollegeAtlas({ manifest, vault, chamberId, onClose }: PluginProps) {
  const [query, setQuery] = useState("");
  const [shortlist, setShortlist] = useState<PluginRecord[]>([]);
  const [busy, setBusy] = useState(false);

  async function refresh() {
    setShortlist(await api.pluginRecordList(vault.id, chamberId, manifest.id, COLLECTION));
  }

  useEffect(() => {
    refresh();
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [chamberId]);

  const listed = useMemo(() => {
    const q = query.trim().toLowerCase();
    if (!q) return COLLEGES;
    return COLLEGES.filter(
      (c) =>
        c.name.toLowerCase().includes(q) ||
        c.location.toLowerCase().includes(q) ||
        c.type.toLowerCase().includes(q) ||
        c.tags.some((t) => t.toLowerCase().includes(q)),
    );
  }, [query]);

  const shortlisted = new Set(shortlist.map((r) => String(r.data.college)));

  async function add(c: College) {
    if (shortlisted.has(c.name) || busy) return;
    setBusy(true);
    try {
      await api.pluginRecordAdd(vault.id, chamberId, manifest.id, COLLECTION, {
        college: c.name,
        tier: suggestTier(c.admitRate),
        location: c.location,
      });
      await refresh();
    } finally {
      setBusy(false);
    }
  }

  async function setTier(rec: PluginRecord, tier: Tier) {
    setShortlist((prev) => prev.map((r) => (r.id === rec.id ? { ...r, data: { ...r.data, tier } } : r)));
    // Records are immutable in the store API; re-add with the new tier.
    await api.pluginRecordDelete(vault.id, rec.id);
    await api.pluginRecordAdd(vault.id, chamberId, manifest.id, COLLECTION, { ...rec.data, tier });
    await refresh();
  }

  async function remove(id: string) {
    setShortlist((prev) => prev.filter((r) => r.id !== id));
    await api.pluginRecordDelete(vault.id, id);
  }

  return (
    <div className="fixed inset-0 z-40 flex items-center justify-center bg-black/50 p-4" onClick={onClose}>
      <div
        onClick={(e) => e.stopPropagation()}
        className="flex h-[680px] w-full max-w-5xl flex-col overflow-hidden rounded-2xl border border-border bg-surface-raised shadow-2xl"
      >
        <div className="flex items-center justify-between border-b border-border px-5 py-3">
          <div>
            <h2 className="font-serif text-lg text-ink">College Landscape Atlas</h2>
            <p className="text-xs text-ink-muted">{vault.name} · browse and build a balanced list</p>
          </div>
          <button onClick={onClose} className="text-ink-muted hover:text-ink" aria-label="Close">
            ✕
          </button>
        </div>

        <div className="grid flex-1 grid-cols-[1fr_320px] overflow-hidden">
          {/* Catalog */}
          <div className="flex flex-col overflow-hidden border-r border-border">
            <div className="border-b border-border p-3">
              <input
                value={query}
                onChange={(e) => setQuery(e.target.value)}
                placeholder="Search by name, place, type, or tag (e.g. 'CS', 'Public', 'Boston')"
                className="w-full rounded-lg border border-border bg-surface px-3 py-1.5 text-sm text-ink outline-none focus:border-primary"
              />
            </div>
            <div className="flex-1 space-y-2 overflow-y-auto p-3">
              {listed.map((c) => (
                <div
                  key={c.name}
                  className="flex items-center justify-between gap-3 rounded-xl border border-border bg-surface p-3"
                >
                  <div className="min-w-0">
                    <div className="flex items-center gap-2">
                      <h3 className="font-serif text-sm text-ink">{c.name}</h3>
                      <span
                        className={`rounded-full border px-2 py-0.5 text-[10px] font-medium ${TIER_STYLE[suggestTier(c.admitRate)]}`}
                      >
                        {suggestTier(c.admitRate)}
                      </span>
                    </div>
                    <p className="text-xs text-ink-muted">
                      {c.location} · {c.type} · {c.admitRate}% admit
                    </p>
                    <div className="mt-1 flex flex-wrap gap-1">
                      {c.tags.map((t) => (
                        <span key={t} className="rounded-full border border-border px-2 py-0.5 text-[10px] text-ink-muted">
                          {t}
                        </span>
                      ))}
                    </div>
                  </div>
                  <button
                    onClick={() => add(c)}
                    disabled={shortlisted.has(c.name) || busy}
                    className="shrink-0 rounded-lg border border-primary px-3 py-1.5 text-xs font-medium text-primary disabled:border-border disabled:text-ink-muted"
                  >
                    {shortlisted.has(c.name) ? "Added" : "+ Shortlist"}
                  </button>
                </div>
              ))}
              {listed.length === 0 && <p className="text-sm text-ink-muted">No matches.</p>}
            </div>
          </div>

          {/* Shortlist grouped by tier */}
          <div className="overflow-y-auto p-3">
            <h3 className="mb-2 px-1 text-xs font-semibold uppercase tracking-wide text-ink-muted">
              Shortlist ({shortlist.length})
            </h3>
            {TIERS.map((tier) => {
              const group = shortlist.filter((r) => String(r.data.tier) === tier);
              return (
                <div key={tier} className="mb-3">
                  <div className={`mb-1 inline-block rounded-full border px-2 py-0.5 text-[10px] font-medium ${TIER_STYLE[tier]}`}>
                    {tier} · {group.length}
                  </div>
                  <ul className="space-y-1.5">
                    {group.map((r) => (
                      <li key={r.id} className="rounded-lg border border-border bg-surface p-2.5">
                        <div className="flex items-start justify-between gap-2">
                          <div className="min-w-0">
                            <div className="truncate text-sm text-ink">{String(r.data.college)}</div>
                            <div className="text-[11px] text-ink-muted">{String(r.data.location ?? "")}</div>
                          </div>
                          <button
                            onClick={() => remove(r.id)}
                            className="text-ink-muted hover:text-red-400"
                            aria-label="Remove"
                          >
                            ✕
                          </button>
                        </div>
                        <div className="mt-1.5 flex gap-1">
                          {TIERS.map((t) => (
                            <button
                              key={t}
                              onClick={() => setTier(r, t)}
                              className={`rounded px-1.5 py-0.5 text-[10px] ${
                                String(r.data.tier) === t
                                  ? "bg-primary text-surface-raised"
                                  : "border border-border text-ink-muted hover:text-ink"
                              }`}
                            >
                              {t[0]}
                            </button>
                          ))}
                        </div>
                      </li>
                    ))}
                  </ul>
                </div>
              );
            })}
            {shortlist.length === 0 && (
              <p className="px-1 text-sm text-ink-muted">Add colleges to build a balanced list.</p>
            )}
          </div>
        </div>
      </div>
    </div>
  );
}
