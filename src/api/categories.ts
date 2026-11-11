import type { Category } from "@/bindings/Category";
import type { CategoryFields } from "@/bindings/CategoryFields";
import type { CategoryList } from "@/bindings/CategoryList";
import type { NewCategory } from "@/bindings/NewCategory";
import { call } from "./client";

/** Every category, archived included, in display order, and each kind's default color. */
export const listCategories = () => call<CategoryList>("list_categories");

export const createCategory = (category: NewCategory) =>
  call<Category>("create_category", { category });

/** The kind can't change once created. */
export const updateCategory = ({
  id,
  category,
}: {
  id: number;
  category: CategoryFields;
}) => call<Category>("update_category", { id, category });

/** Each id's position becomes its sort order. Send one kind's list at a time. */
export const reorderCategories = (ids: number[]) =>
  call<null>("reorder_categories", { ids });

/** The last active category of a kind is a `conflict`. */
export const archiveCategory = (id: number) =>
  call<null>("archive_category", { id });

export const unarchiveCategory = (id: number) =>
  call<null>("unarchive_category", { id });

/** Only a category no transaction or recurring rule uses (`inUse` false) can be deleted. */
export const deleteCategory = (id: number) =>
  call<null>("delete_category", { id });

/**
 * Moves every transaction of `fromId` to the active category `intoId` of the same kind, then
 * deletes `fromId` (UI_FLOWS F7).
 */
export const mergeCategories = ({
  fromId,
  intoId,
}: {
  fromId: number;
  intoId: number;
}) => call<null>("merge_categories", { fromId, intoId });
