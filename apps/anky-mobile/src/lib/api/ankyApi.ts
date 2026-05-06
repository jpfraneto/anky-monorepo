import {
  AppConfigResponse,
  assertCreditReceipt,
  BackendAuthResponse,
  CreateCheckoutRequest,
  CreateCheckoutResponse,
  CreateProcessingTicketRequest,
  CreateProcessingTicketResponse,
  CreditBalanceResponse,
  MobileCreditResponse,
  MobileLoomLookupResponse,
  MobileMintAuthorizationRequest,
  MobileMintAuthorizationResponse,
  PrepareMobileLoomMintRequest,
  PrepareMobileLoomMintResponse,
  MobileReflectionJobResponse,
  MobileReflectionRequest,
  MobileReflectionResponse,
  MobileSealScoreResponse,
  MobileSolanaConfigResponse,
  MobileSpendCreditsRequest,
  MobileSpendCreditsResponse,
  PrivyAuthRequest,
  RecordMobileLoomMintRequest,
  RecordMobileLoomMintResponse,
  RecordMobileSealRequest,
  RecordMobileSealResponse,
  RunProcessingRequest,
  RunProcessingResponse,
  SealLookupQuery,
  SealLookupResponse,
  SendThreadMessageRequest,
  SendThreadMessageResponse,
} from "./types";

type FetchImpl = typeof fetch;

export type AnkyApiClientOptions = {
  baseUrl: string;
  fetchImpl?: FetchImpl;
  timeoutMs?: number;
};

export class AnkyApiError extends Error {
  readonly body?: string;
  readonly path: string;
  readonly status: number;

  constructor({
    body,
    path,
    status,
  }: {
    body?: string;
    path: string;
    status: number;
  }) {
    super(`Anky API request to ${path} failed with HTTP ${status}.`);
    this.body = body;
    this.name = "AnkyApiError";
    this.path = path;
    this.status = status;
  }
}

export class AnkyApiClient {
  private readonly baseUrl: string;
  private readonly fetchImpl: FetchImpl;
  private readonly timeoutMs: number;

  constructor({ baseUrl, fetchImpl = fetch, timeoutMs = 30000 }: AnkyApiClientOptions) {
    if (baseUrl.trim().length === 0) {
      throw new Error("Anky API baseUrl is required.");
    }

    this.baseUrl = baseUrl.replace(/\/+$/, "");
    this.fetchImpl = fetchImpl;
    this.timeoutMs = timeoutMs;
    this.prepareMobileLoomMint = this.prepareMobileLoomMint.bind(this);
  }

  getConfig(): Promise<AppConfigResponse> {
    return this.request<AppConfigResponse>("/api/v1/config");
  }

  getCreditBalance(): Promise<CreditBalanceResponse> {
    return this.request<CreditBalanceResponse>("/api/v1/credits/balance");
  }

  getMobileSolanaConfig(): Promise<MobileSolanaConfigResponse> {
    return this.request<MobileSolanaConfigResponse>("/api/mobile/solana/config");
  }

  getMobileCreditBalance(identityId: string): Promise<MobileCreditResponse> {
    const params = new URLSearchParams({ identityId });

    return this.request<MobileCreditResponse>(`/api/mobile/credits?${params.toString()}`);
  }

  spendMobileCredits(
    request: MobileSpendCreditsRequest,
  ): Promise<MobileSpendCreditsResponse> {
    return this.request<MobileSpendCreditsResponse>("/api/mobile/credits/spend", {
      body: JSON.stringify(request),
      method: "POST",
    });
  }

  createMobileMintAuthorization(
    request: MobileMintAuthorizationRequest,
  ): Promise<MobileMintAuthorizationResponse> {
    return this.request<MobileMintAuthorizationResponse>("/api/mobile/looms/mint-authorizations", {
      body: JSON.stringify(request),
      method: "POST",
    });
  }

  prepareMobileLoomMint(
    request: PrepareMobileLoomMintRequest,
  ): Promise<PrepareMobileLoomMintResponse> {
    return this.request<PrepareMobileLoomMintResponse>("/api/mobile/looms/prepare-mint", {
      body: JSON.stringify(request),
      method: "POST",
    });
  }

  recordMobileLoomMint(
    request: RecordMobileLoomMintRequest,
  ): Promise<RecordMobileLoomMintResponse> {
    return this.request<RecordMobileLoomMintResponse>("/api/mobile/looms/record", {
      body: JSON.stringify(request),
      method: "POST",
    });
  }

  lookupMobileLooms(wallet: string): Promise<MobileLoomLookupResponse> {
    const params = new URLSearchParams({ wallet });

    return this.request<MobileLoomLookupResponse>(`/api/mobile/looms?${params.toString()}`);
  }

  exchangePrivyAuthToken(request: PrivyAuthRequest): Promise<BackendAuthResponse> {
    return this.request<BackendAuthResponse>("/swift/v1/auth/privy", {
      body: JSON.stringify(request),
      method: "POST",
    });
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

  createMobileReflection(
    request: MobileReflectionRequest,
  ): Promise<MobileReflectionResponse> {
    return this.request<MobileReflectionResponse>("/api/mobile/reflections", {
      body: JSON.stringify(request),
      method: "POST",
    });
  }

  getMobileReflection(jobId: string): Promise<MobileReflectionJobResponse> {
    return this.request<MobileReflectionJobResponse>(
      `/api/mobile/reflections/${encodeURIComponent(jobId)}`,
    );
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

  lookupMobileSeals(query: SealLookupQuery): Promise<SealLookupResponse> {
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

    return this.request<SealLookupResponse>(`/api/mobile/seals?${params.toString()}`);
  }

  lookupMobileSealScore(wallet: string): Promise<MobileSealScoreResponse> {
    const params = new URLSearchParams({ wallet });

    return this.request<MobileSealScoreResponse>(
      `/api/mobile/seals/score?${params.toString()}`,
    );
  }

  recordMobileSeal(request: RecordMobileSealRequest): Promise<RecordMobileSealResponse> {
    return this.request<RecordMobileSealResponse>("/api/mobile/seals/record", {
      body: JSON.stringify(request),
      method: "POST",
    });
  }

  sendThreadMessage(
    request: SendThreadMessageRequest,
  ): Promise<SendThreadMessageResponse> {
    return this.request<SendThreadMessageResponse>("/api/mobile/threads", {
      body: JSON.stringify(request),
      method: "POST",
    });
  }

  private async request<T>(path: string, init: RequestInit = {}): Promise<T> {
    const headers = new Headers(init.headers);

    if (init.body != null && !headers.has("content-type")) {
      headers.set("content-type", "application/json");
    }

    const controller = typeof AbortController === "undefined" ? null : new AbortController();
    const timeout =
      controller == null
        ? null
        : setTimeout(() => {
            controller.abort();
          }, this.timeoutMs);

    let response: Response;

    try {
      response = await this.fetchImpl(`${this.baseUrl}${path}`, {
        ...init,
        headers,
        signal: init.signal ?? controller?.signal,
      });
    } finally {
      if (timeout != null) {
        clearTimeout(timeout);
      }
    }

    if (!response.ok) {
      let body: string | undefined;

      try {
        body = await response.text();
      } catch {
        body = undefined;
      }

      throw new AnkyApiError({ body, path, status: response.status });
    }

    return (await response.json()) as T;
  }
}

export function createAnkyApiClient(options: AnkyApiClientOptions): AnkyApiClient {
  return new AnkyApiClient(options);
}
