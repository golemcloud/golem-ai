declare module 'search-library' {
  import * as golemSearch100Types from 'golem:search/types@1.0.0';
  export namespace types {
    export type SearchError = {
      tag: 'index-not-found'
    } |
    {
      tag: 'invalid-query'
      val: string
    } |
    {
      tag: 'unsupported'
    } |
    {
      tag: 'internal'
      val: string
    } |
    {
      tag: 'timeout'
    } |
    {
      tag: 'rate-limited'
    };
    export type IndexName = string;
    export type DocumentId = string;
    export type Json = string;
    export type Doc = {
      id: DocumentId;
      content: Json;
    };
    export type HighlightConfig = {
      fields: string[];
      preTag: string | undefined;
      postTag: string | undefined;
      maxLength: number | undefined;
    };
    export type SearchConfig = {
      timeoutMs: number | undefined;
      boostFields: [string, number][];
      attributesToRetrieve: string[];
      language: string | undefined;
      typoTolerance: boolean | undefined;
      exactMatchBoost: number | undefined;
      providerParams: Json | undefined;
    };
    export type SearchQuery = {
      q: string | undefined;
      filters: string[];
      sort: string[];
      facets: string[];
      page: number | undefined;
      perPage: number | undefined;
      offset: number | undefined;
      highlight: HighlightConfig | undefined;
      config: SearchConfig | undefined;
    };
    export type SearchHit = {
      id: DocumentId;
      score: number | undefined;
      content: Json | undefined;
      highlights: Json | undefined;
    };
    export type SearchResults = {
      total: number | undefined;
      page: number | undefined;
      perPage: number | undefined;
      hits: SearchHit[];
      facets: Json | undefined;
      tookMs: number | undefined;
    };
    export type FieldType = "text" | "keyword" | "integer" | "float" | "boolean" | "date" | "geo-point";
    export type SchemaField = {
      name: string;
      fieldType: FieldType;
      required: boolean;
      facet: boolean;
      sort: boolean;
      index: boolean;
    };
    export type Schema = {
      fields: SchemaField[];
      primaryKey: string | undefined;
    };
  }
  export namespace core {
    export function createIndex(name: IndexName, schema: Schema | undefined): Promise<Result<void, SearchError>>;
    export function deleteIndex(name: IndexName): Promise<Result<void, SearchError>>;
    export function listIndexes(): Promise<Result<IndexName[], SearchError>>;
    export function upsert(index: IndexName, doc: Doc): Promise<Result<void, SearchError>>;
    export function upsertMany(index: IndexName, docs: Doc[]): Promise<Result<void, SearchError>>;
    export function delete_(index: IndexName, id: DocumentId): Promise<Result<void, SearchError>>;
    export function deleteMany(index: IndexName, ids: DocumentId[]): Promise<Result<void, SearchError>>;
    export function get(index: IndexName, id: DocumentId): Promise<Result<Doc | undefined, SearchError>>;
    export function search(index: IndexName, query: SearchQuery): Promise<Result<SearchResults, SearchError>>;
    export function streamSearch(index: IndexName, query: SearchQuery): Promise<Result<SearchStream, SearchError>>;
    export function getSchema(index: IndexName): Promise<Result<Schema, SearchError>>;
    export function updateSchema(index: IndexName, schema: Schema): Promise<Result<void, SearchError>>;
    export class SearchStream {
      async getNext(): Promise<SearchHit[] | undefined>;
      async blockingGetNext(): Promise<SearchHit[]>;
    }
    export type IndexName = golemSearch100Types.IndexName;
    export type DocumentId = golemSearch100Types.DocumentId;
    export type Doc = golemSearch100Types.Doc;
    export type SearchQuery = golemSearch100Types.SearchQuery;
    export type SearchResults = golemSearch100Types.SearchResults;
    export type SearchHit = golemSearch100Types.SearchHit;
    export type Schema = golemSearch100Types.Schema;
    export type SearchError = golemSearch100Types.SearchError;
    export type Result<T, E> = { tag: 'ok', val: T } | { tag: 'err', val: E };
  }
}
