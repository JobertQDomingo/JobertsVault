import { QueryClient } from "@tanstack/react-query";

/**
 * Errors come from Rust and are the same on a second try, so a failed load shows its error at
 * once instead of after three retries. Refetching on window focus stays on (ARCHITECTURE §5.2).
 */
export const createQueryClient = () =>
  new QueryClient({ defaultOptions: { queries: { retry: false } } });

export const queryClient = createQueryClient();
