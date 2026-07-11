import type { ComponentType } from "react";
import type { PluginManifest, VaultCard } from "../lib/ipc";
import { EssayVersionControl } from "../features/tools/EssayVersionControl";
import { TimelineWeaver } from "../features/tools/TimelineWeaver";
import { PathwayBlueprint } from "../features/tools/PathwayBlueprint";
import { CollegeAtlas } from "../features/tools/CollegeAtlas";
import { NarrativeLoom } from "../features/tools/NarrativeLoom";

/** Props every plugin component (native or declarative) receives. */
export interface PluginProps {
  manifest: PluginManifest;
  vault: VaultCard;
  chamberId: string;
  /** This plugin's per-vault settings (shape defined by its config schema). */
  settings: unknown;
  onClose: () => void;
}

/**
 * Maps a manifest's native `component` key to its React implementation.
 * Built-in plugins live here; declarative Forge plugins render generically and
 * are not in this table.
 */
const NATIVE: Record<string, ComponentType<PluginProps>> = {
  EssayVersionControl,
  TimelineWeaver,
  PathwayBlueprint,
  CollegeAtlas,
  NarrativeLoom,
};

/** Look up a native plugin component by its manifest key. */
export function nativeComponent(key: string): ComponentType<PluginProps> | undefined {
  return NATIVE[key];
}
