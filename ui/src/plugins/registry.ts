import type { ComponentType } from "react";
import type { PluginManifest, VaultCard } from "../lib/ipc";
import { EssayVersionControl } from "../features/tools/EssayVersionControl";
import { TimelineWeaver } from "../features/tools/TimelineWeaver";

/** Props every plugin component (native or declarative) receives. */
export interface PluginProps {
  manifest: PluginManifest;
  vault: VaultCard;
  chamberId: string;
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
};

/** Look up a native plugin component by its manifest key. */
export function nativeComponent(key: string): ComponentType<PluginProps> | undefined {
  return NATIVE[key];
}
