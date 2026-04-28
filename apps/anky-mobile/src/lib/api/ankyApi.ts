import {
  AppConfigResponse,
  assertCreditReceipt,
  CreateCheckoutRequest,
  CreateCheckoutResponse,
  CreateProcessingTicketRequest,
  CreateProcessingTicketResponse,
  CreditBalanceResponse,
  RunProcessingRequest,
  RunProcessingResponse,
  SealLookupQuery,
  SealLookupResponse,
} from "./types";

type FetchImpl = typeof fetch;

export type AnkyApiClientOptions = {
  baseUrl: string;
  fetchImpl?: FetchImpl;
};

export class AnkyApiClient {
  private readonly baseUrl: string;
  private readonly fetchImpl: FetchImpl;

  constructor({ baseUrl, fetchImpl = fetch }: AnkyApiClientOptions) {
    if (baseUrl.trim().length === 0) {
      throw new Error("Anky API baseUrl is required.");
    }

    this.baseUrl = baseUrl.replace(/\/+$/, "");
    this.fetchImpl = fetchImpl;
  }

  getConfig(): Promise<AppConfigResponse> {
    return this.request<AppConfigResponse>("/api/v1/config");
  }

  getCreditBalance(): Promise<CreditBalanceResponse> {
    return this.request<CreditBalanceResponse>("/api/v1/credits/balance");
  }

  createCheckoutSession(
    request: CreateCheckoutRequest,
  ): Promise<CreateCheckoutResponse> {
    return this.request<CreateCheckoutResponse>("/api/v1/credits/checkout", {
      body: JSON.stringify(request),
      method: "POST",
    });
  }

  async createProcessingTicket(
    request: CreateProcessingTicketRequest,
  ): Promise<CreateProcessingTicketResponse> {
    const response = await this.request<CreateProcessingTicketResponse>("/api/v1/processing/tickets", {
      body: JSON.stringify(request),
      method: "POST",
    });

    assertCreditReceipt(response.receipt);

    return response;
  }

  runProcessing(request: RunProcessingRequest): Promise<RunProcessingResponse> {
    assertCreditReceipt(request.receipt);

    return this.request<RunProcessingResponse>("/api/v1/processing/run", {
      body: JSON.stringify(request),
      method: "POST",
    });
  }

  lookupSeals(query: SealLookupQuery): Promise<SealLookupResponse> {
    const params = new URLSearchParams();

    if ("wallet" in query && query.wallet != null) {
      params.set("wallet", query.wallet);
    }

    if ("loomId" in query && query.loomId != null) {
      params.set("loomId", query.loomId);
    }

    if ("sessionHash" in query && query.sessionHash != null) {
      params.set("sessionHash", query.sessionHash);
    }

    return this.request<SealLookupResponse>(`/api/v1/seals?${params.toString()}`);
  }

  private async request<T>(path: string, init: RequestInit = {}): Promise<T> {
    const headers = new Headers(init.headers);

    if (init.body != null && !headers.has("content-type")) {
      headers.set("content-type", "application/json");
    }

    const response = await this.fetchImpl(`${this.baseUrl}${path}`, {
      ...init,
      headers,
    });

    if (!response.ok) {
      throw new Error(`Anky API request failed with HTTP ${response.status}.`);
    }

    return (await response.json()) as T;
  }
}

export function createAnkyApiClient(options: AnkyApiClientOptions): AnkyApiClient {
  return new AnkyApiClient(options);
}
