import { useQuery } from "@tanstack/react-query";
import {
  archiveCategory,
  createCategory,
  deleteCategory,
  listCategories,
  mergeCategories,
  reorderCategories,
  unarchiveCategory,
  updateCategory,
} from "@/api/categories";
import { queryKeys } from "@/lib/queryKeys";
import { useLedgerMutation } from "./useLedgerMutation";

export const useCategories = () =>
  useQuery({ queryKey: queryKeys.categories, queryFn: listCategories });

export const useCreateCategory = () => useLedgerMutation(createCategory);
export const useUpdateCategory = () => useLedgerMutation(updateCategory);
export const useReorderCategories = () => useLedgerMutation(reorderCategories);
export const useArchiveCategory = () => useLedgerMutation(archiveCategory);
export const useUnarchiveCategory = () => useLedgerMutation(unarchiveCategory);
export const useDeleteCategory = () => useLedgerMutation(deleteCategory);
export const useMergeCategories = () => useLedgerMutation(mergeCategories);
