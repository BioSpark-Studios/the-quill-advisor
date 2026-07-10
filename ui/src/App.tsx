import { useState } from "react";
import { Splash } from "./features/splash/Splash";
import { MasterVault } from "./features/vault/MasterVault";
import { VaultView } from "./features/vault/VaultView";
import type { VaultCard } from "./lib/ipc";

type Screen = "splash" | "master" | "vault";

export default function App() {
  const [screen, setScreen] = useState<Screen>("splash");
  const [openVault, setOpenVault] = useState<VaultCard | null>(null);

  if (screen === "splash") {
    return <Splash onEnter={() => setScreen("master")} />;
  }

  if (screen === "vault" && openVault) {
    return <VaultView vault={openVault} onBack={() => setScreen("master")} />;
  }

  return (
    <MasterVault
      onOpenVault={(v) => {
        setOpenVault(v);
        setScreen("vault");
      }}
    />
  );
}
