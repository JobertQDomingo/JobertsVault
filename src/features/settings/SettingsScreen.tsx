import { useSearchParams } from "react-router";
import { ErrorState } from "@/components/ErrorState";
import { GlassPanel } from "@/components/GlassPanel";
import { Loading, Skeleton } from "@/components/Loading";
import { Tabs, TabsContent, TabsList, TabsTrigger } from "@/components/ui/tabs";
import { useSettings } from "@/hooks/useSettings";
import { CategoriesTab } from "./CategoriesTab";
import { GeneralTab } from "./GeneralTab";
import { PaymentMethodsTab } from "./PaymentMethodsTab";

const TABS = ["general", "categories", "methods"] as const;
type Tab = (typeof TABS)[number];

/**
 * `/settings` (UI_FLOWS §6): General, Categories and Payment methods, the tab kept in `?tab=` so
 * Back and reloads keep it. The Backup (P2) and Appearance (P4) tabs come with their phases.
 */
export function SettingsScreen() {
  const [params, setParams] = useSearchParams();
  const asked = params.get("tab");
  const tab: Tab = TABS.find((t) => t === asked) ?? "general";
  const settings = useSettings();

  return (
    <GlassPanel className="px-5">
      <Tabs
        value={tab}
        onValueChange={(next) =>
          setParams(next === "general" ? {} : { tab: next }, { replace: true })
        }
      >
        <TabsList className="mb-2">
          <TabsTrigger value="general">General</TabsTrigger>
          <TabsTrigger value="categories">Categories</TabsTrigger>
          <TabsTrigger value="methods">Payment methods</TabsTrigger>
        </TabsList>
        <TabsContent value="general" className="outline-none">
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
        <TabsContent value="categories" className="outline-none">
          <CategoriesTab />
        </TabsContent>
        <TabsContent value="methods" className="outline-none">
          <PaymentMethodsTab />
        </TabsContent>
      </Tabs>
    </GlassPanel>
  );
}
