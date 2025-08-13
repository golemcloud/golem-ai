declare module 'exec-library' {
  import * as golemExec100Types from 'golem:exec/types@1.0.0';
  export namespace types {
    export type LanguageKind = "javascript" | "python";
    export type Language = {
      kind: LanguageKind;
      version: string | undefined;
    };
    export type Encoding = "utf8" | "base64" | "hex";
    export type File = {
      name: string;
      content: number[];
      encoding: Encoding | undefined;
    };
    export type Limits = {
      timeMs: bigint | undefined;
      memoryBytes: bigint | undefined;
      fileSizeBytes: bigint | undefined;
      maxProcesses: number | undefined;
    };
    export type StageResult = {
      stdout: string;
      stderr: string;
      exitCode: number | undefined;
      signal: string | undefined;
    };
    export type ExecResult = {
      compile: StageResult | undefined;
      run: StageResult;
      timeMs: bigint | undefined;
      memoryBytes: bigint | undefined;
    };
    export type Error = {
      tag: 'unsupported-language'
    } |
    {
      tag: 'compilation-failed'
      val: StageResult
    } |
    {
      tag: 'runtime-failed'
      val: StageResult
    } |
    {
      tag: 'timeout'
    } |
    {
      tag: 'resource-exceeded'
    } |
    {
      tag: 'internal'
      val: string
    };
  }
  export namespace executor {
    export function run(lang: Language, snippet: string, modules: File[], stdin: string | undefined, args: string[], env: [string, string][], constraints: Limits | undefined): Promise<Result<ExecResult, Error>>;
    export class Session {
      constructor(lang: Language, modules: File[]);
      async upload(file: File): Promise<Result<void, Error>>;
      async run(snippet: string, args: string[], stdin: string | undefined, env: [string, string][], constraints: Limits | undefined): Promise<Result<ExecResult, Error>>;
      async download(path: string): Promise<Result<number[], Error>>;
      async listFiles(dir: string): Promise<Result<string[], Error>>;
      async setWorkingDir(path: string): Promise<Result<void, Error>>;
    }
    export type Language = golemExec100Types.Language;
    export type File = golemExec100Types.File;
    export type Limits = golemExec100Types.Limits;
    export type ExecResult = golemExec100Types.ExecResult;
    export type Error = golemExec100Types.Error;
    export type Result<T, E> = { tag: 'ok', val: T } | { tag: 'err', val: E };
  }
}
