import { writable } from "svelte/store";

export type SearchState = {
  query: string;
  mode: "live" | "user";
  results: any[];
  searched: boolean;
  userHasMore: boolean;
  currentPage: number | null;
  nextPage: number | null;
  previousPage: number | null;
  totalEntries: number | null;
  currentCursor: string | null;
  nextCursor: string | null;
  recommendUsers: any[];
  recommendPage: number;
  recommendHasMore: boolean;
  selectedUser: any;
  selectedUserDetail: any;
  selectedUserLiveHistory: any[];
  userHistoryPage: number;
  userHistoryHasMore: boolean;
  userHistoryTotal: number | null;
  userHistoryCurrentPage: number | null;
  userHistoryNextPage: number | null;
  userHistoryPreviousPage: number | null;
  userDetailError: string;
  userHistoryError: string;
  error: string;
  recommendError: string;
};

export const searchState = writable<SearchState>({
  query: "",
  mode: "live",
  results: [],
  searched: false,
  userHasMore: true,
  currentPage: null,
  nextPage: null,
  previousPage: null,
  totalEntries: null,
  currentCursor: null,
  nextCursor: null,
  recommendUsers: [],
  recommendPage: 1,
  recommendHasMore: true,
  selectedUser: null,
  selectedUserDetail: null,
  selectedUserLiveHistory: [],
  userHistoryPage: 1,
  userHistoryHasMore: true,
  userHistoryTotal: null,
  userHistoryCurrentPage: null,
  userHistoryNextPage: null,
  userHistoryPreviousPage: null,
  userDetailError: "",
  userHistoryError: "",
  error: "",
  recommendError: ""
});
