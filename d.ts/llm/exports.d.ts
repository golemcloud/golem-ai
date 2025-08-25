declare module 'llm-library' {
  export namespace llm {
    export function send(messages: Message[], config: Config): Promise<ChatEvent>;
    export function continue_(messages: Message[], toolResults: [ToolCall, ToolResult][], config: Config): Promise<ChatEvent>;
    export function stream(messages: Message[], config: Config): Promise<ChatStream>;
    export class ChatStream {
      async getNext(): Promise<StreamEvent[] | undefined>;
      async blockingGetNext(): Promise<StreamEvent[]>;
    }
    export type Role = "user" | "assistant" | "system" | "tool";
    export type ErrorCode = "invalid-request" | "authentication-failed" | "rate-limit-exceeded" | "internal-error" | "unsupported" | "unknown";
    export type FinishReason = "stop" | "length" | "tool-calls" | "content-filter" | "error" | "other";
    export type ImageDetail = "low" | "high" | "auto";
    export type ImageUrl = {
      url: string;
      detail: ImageDetail | undefined;
    };
    export type ImageSource = {
      data: number[];
      mimeType: string;
      detail: ImageDetail | undefined;
    };
    export type ImageReference = {
      tag: 'url'
      val: ImageUrl
    } |
    {
      tag: 'inline'
      val: ImageSource
    };
    export type ContentPart = {
      tag: 'text'
      val: string
    } |
    {
      tag: 'image'
      val: ImageReference
    };
    export type Message = {
      role: Role;
      name: string | undefined;
      content: ContentPart[];
    };
    export type ToolDefinition = {
      name: string;
      description: string | undefined;
      parametersSchema: string;
    };
    export type ToolCall = {
      id: string;
      name: string;
      argumentsJson: string;
    };
    export type ToolSuccess = {
      id: string;
      name: string;
      resultJson: string;
      executionTimeMs: number | undefined;
    };
    export type ToolFailure = {
      id: string;
      name: string;
      errorMessage: string;
      errorCode: string | undefined;
    };
    export type ToolResult = {
      tag: 'success'
      val: ToolSuccess
    } |
    {
      tag: 'error'
      val: ToolFailure
    };
    export type Kv = {
      key: string;
      value: string;
    };
    export type Config = {
      model: string;
      temperature: number | undefined;
      maxTokens: number | undefined;
      stopSequences: string[] | undefined;
      tools: ToolDefinition[];
      toolChoice: string | undefined;
      providerOptions: Kv[];
    };
    export type Usage = {
      inputTokens: number | undefined;
      outputTokens: number | undefined;
      totalTokens: number | undefined;
    };
    export type ResponseMetadata = {
      finishReason: FinishReason | undefined;
      usage: Usage | undefined;
      providerId: string | undefined;
      timestamp: string | undefined;
      providerMetadataJson: string | undefined;
    };
    export type CompleteResponse = {
      id: string;
      content: ContentPart[];
      toolCalls: ToolCall[];
      metadata: ResponseMetadata;
    };
    export type Error = {
      code: ErrorCode;
      message: string;
      providerErrorJson: string | undefined;
    };
    export type ChatEvent = {
      tag: 'message'
      val: CompleteResponse
    } |
    {
      tag: 'tool-request'
      val: ToolCall[]
    } |
    {
      tag: 'error'
      val: Error
    };
    export type StreamDelta = {
      content: ContentPart[] | undefined;
      toolCalls: ToolCall[] | undefined;
    };
    export type StreamEvent = {
      tag: 'delta'
      val: StreamDelta
    } |
    {
      tag: 'finish'
      val: ResponseMetadata
    } |
    {
      tag: 'error'
      val: Error
    };
  }
}
