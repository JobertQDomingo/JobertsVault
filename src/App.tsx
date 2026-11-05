import { AuroraBackground } from "./components/AuroraBackground";
import { GlassPanel } from "./components/GlassPanel";

export default function App() {
  return (
    <>
      <AuroraBackground />
      <main className="flex h-full items-center justify-center">
        <GlassPanel className="px-8 py-6">
          <h1 className="text-h1">Jobert's Vault</h1>
        </GlassPanel>
      </main>
    </>
  );
}
