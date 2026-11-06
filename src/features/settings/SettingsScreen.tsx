import { useSearchParams } from "react-router";
import { ErrorState } from "@/components/ErrorState";
import { GlassPanel } from "@/components/GlassPanel";
import { Loading, Skeleton } from "@/components/Loading";
import { Tabs, TabsContent, TabsList, TabsTrigger } from "@/components/ui/tabs";
import { useSettings } from "@/hooks/useSettings";
import { BackupTab } from "./BackupTab";
import { CategoriesTab } from "./CategoriesTab";
import { GeneralTab } from "./GeneralTab";
import { PaymentMethodsTab } from "./PaymentMethodsTab";

const TABS = ["general", "categories", "methods", "backup"] as const;
const CONTENT = "min-h-0 flex-1 overflow-y-auto px-5 outline-none";
type Tab = (typeof TABS)[number];

/**
 * `/settings` (UI_FLOWS §6): General, Categories, Payment methods and Backup, the tab kept in
 * `?tab=` so Back and reloads keep it. The Appearance tab (P4) comes with its phase.
 */
export function SettingsScreen() {
  const [params, setParams] = useSearchParams();
  const asked = params.get("tab");
  const tab: Tab = TABS.find((t) => t === asked) ?? "general";
  const settings = useSettings();

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
        </TabsList>
        <TabsContent value="general" className={CONTENT}>
          {settings.data ? (
            <GeneralTab settings={settings.data} />
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
          )}
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
      </Tabs>
    </GlassPanel>
  );
}
