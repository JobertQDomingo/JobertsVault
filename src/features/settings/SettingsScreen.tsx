import type { ReactNode } from "react";
import { useSearchParams } from "react-router";
import type { SettingsView } from "@/bindings/SettingsView";
import { ErrorState } from "@/components/ErrorState";
import { GlassPanel } from "@/components/GlassPanel";
import { Loading, Skeleton } from "@/components/Loading";
import { Tabs, TabsContent, TabsList, TabsTrigger } from "@/components/ui/tabs";
import { useSettings } from "@/hooks/useSettings";
import { AppearanceTab } from "./AppearanceTab";
import { BackupTab } from "./BackupTab";
import { CategoriesTab } from "./CategoriesTab";
import { GeneralTab } from "./GeneralTab";
import { PaymentMethodsTab } from "./PaymentMethodsTab";

const TABS = [
  "general",
  "categories",
  "methods",
  "backup",
  "appearance",
] as const;
const CONTENT = "min-h-0 flex-1 overflow-y-auto px-5 outline-none";
type Tab = (typeof TABS)[number];

/**
 * `/settings` (UI_FLOWS §6): General, Categories, Payment methods, Backup and Appearance, the tab
 * kept in `?tab=` so Back and reloads keep it.
 */
export function SettingsScreen() {
  const [params, setParams] = useSearchParams();
  const asked = params.get("tab");
  const tab: Tab = TABS.find((t) => t === asked) ?? "general";
  const settings = useSettings();
  // General and Appearance show the stored settings, or why they're not there yet.
  const withSettings = (tab: (loaded: SettingsView) => ReactNode) =>
    settings.data ? (
      tab(settings.data)
    ) : settings.isError ? (
      <ErrorState
        onRetry={() => settings.refetch()}
        pending={settings.isFetching}
      />
    ) : (
      <Loading className="grid gap-3 py-3">
        {Array.from({ length: 6 }, (_, i) => (
          <Skeleton key={i} className="h-9" />
        ))}
      </Loading>
    );

  return (
    // The tabs stay; the content scrolls inside the one panel (DESIGN_SYSTEM §11).
    <GlassPanel className="flex min-h-0 flex-1 flex-col overflow-hidden">
      <Tabs
        className="flex min-h-0 flex-1 flex-col"
        value={tab}
        onValueChange={(next) =>
          setParams(next === "general" ? {} : { tab: next }, { replace: true })
        }
      >
        <TabsList className="mx-5 mb-2">
          <TabsTrigger value="general">General</TabsTrigger>
          <TabsTrigger value="categories">Categories</TabsTrigger>
          <TabsTrigger value="methods">Payment methods</TabsTrigger>
          <TabsTrigger value="backup">Backup</TabsTrigger>
          <TabsTrigger value="appearance">Appearance</TabsTrigger>
        </TabsList>
        <TabsContent value="general" className={CONTENT}>
          {withSettings((loaded) => (
            <GeneralTab settings={loaded} />
          ))}
        </TabsContent>
        <TabsContent value="categories" className={CONTENT}>
          <CategoriesTab />
        </TabsContent>
        <TabsContent value="methods" className={CONTENT}>
          <PaymentMethodsTab />
        </TabsContent>
        <TabsContent value="backup" className={CONTENT}>
          <BackupTab />
        </TabsContent>
        <TabsContent value="appearance" className={CONTENT}>
          {withSettings((loaded) => (
            <AppearanceTab settings={loaded} />
          ))}
        </TabsContent>
      </Tabs>
    </GlassPanel>
  );
}
