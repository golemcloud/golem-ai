declare module 'websearch-library' {
  import * as golemWebSearch100Types from 'golem:web-search/types@1.0.0';
  export namespace types {
    export type ImageResult = {
      url: string;
      description: string | undefined;
    };
    export type SearchResult = {
      title: string;
      url: string;
      snippet: string;
      displayUrl: string | undefined;
      source: string | undefined;
      score: number | undefined;
      htmlSnippet: string | undefined;
      datePublished: string | undefined;
      images: ImageResult[] | undefined;
      contentChunks: string[] | undefined;
    };
    export type SafeSearchLevel = "off" | "medium" | "high";
    export type RateLimitInfo = {
      limit: number;
      remaining: number;
      resetTimestamp: bigint;
    };
    export type SearchMetadata = {
      query: string;
      totalResults: bigint | undefined;
      searchTimeMs: number | undefined;
      safeSearch: SafeSearchLevel | undefined;
      language: string | undefined;
      region: string | undefined;
      nextPageToken: string | undefined;
      rateLimits: RateLimitInfo | undefined;
      currentPage: number;
    };
    export type TimeRange = "day" | "week" | "month" | "year";
    export type SearchParams = {
      query: string;
      safeSearch: SafeSearchLevel | undefined;
      language: string | undefined;
      region: string | undefined;
      maxResults: number | undefined;
      timeRange: TimeRange | undefined;
      includeDomains: string[] | undefined;
      excludeDomains: string[] | undefined;
      includeImages: boolean | undefined;
      includeHtml: boolean | undefined;
      advancedAnswer: boolean | undefined;
    };
    export type SearchError = {
      tag: 'invalid-query'
    } |
    {
      tag: 'rate-limited'
      val: number
    } |
    {
      tag: 'unsupported-feature'
      val: string
    } |
    {
      tag: 'backend-error'
      val: string
    };
  }
  export namespace webSearch {
    export function startSearch(params: SearchParams): Promise<Result<SearchSession, SearchError>>;
    export function searchOnce(params: SearchParams): Promise<Result<[SearchResult[], SearchMetadata | undefined], SearchError>>;
    export class SearchSession {
      async nextPage(): Promise<Result<SearchResult[], SearchError>>;
      async getMetadata(): Promise<SearchMetadata | undefined>;
    }
    export type SearchParams = golemWebSearch100Types.SearchParams;
    export type SearchResult = golemWebSearch100Types.SearchResult;
    export type SearchMetadata = golemWebSearch100Types.SearchMetadata;
    export type SearchError = golemWebSearch100Types.SearchError;
    export type Result<T, E> = { tag: 'ok', val: T } | { tag: 'err', val: E };
  }
}
