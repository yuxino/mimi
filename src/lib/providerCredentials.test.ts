import { describe, expect, it } from "vitest";
import {
  buildProviderCredentials,
  credentialEditorStateAfterDeleteRequest,
  credentialFieldsForProvider,
  emptyCredentialDraft,
} from "./providerCredentials";

describe("provider credential payloads", () => {
  it("keeps one-key providers on the compact credential shape", () => {
    const draft = emptyCredentialDraft();
    draft.apiKey = "  sk-test  ";
    expect(buildProviderCredentials("googleGeminiLive", draft)).toEqual({
      kind: "apiKey",
      apiKey: "sk-test",
    });
  });

  it("requires every provider-specific field", () => {
    expect(credentialFieldsForProvider("azureOpenAIRealtime")).toEqual([
      "endpoint",
      "deployment",
      "transcriptionDeployment",
      "apiKey",
    ]);
    const draft = emptyCredentialDraft();
    draft.endpoint = "https://mimi.openai.azure.com";
    draft.deployment = "translate";
    draft.transcriptionDeployment = "transcribe";
    expect(buildProviderCredentials("azureOpenAIRealtime", draft)).toBeNull();
    draft.apiKey = "secret";
    expect(buildProviderCredentials("azureOpenAIRealtime", draft)).toEqual({
      kind: "azureOpenAI",
      endpoint: "https://mimi.openai.azure.com",
      deployment: "translate",
      transcriptionDeployment: "transcribe",
      apiKey: "secret",
    });
  });

  it("describes multi-field Tencent and Baidu credentials", () => {
    expect(credentialFieldsForProvider("tencentCloud")).toEqual([
      "appId",
      "secretId",
      "secretKey",
    ]);
    expect(credentialFieldsForProvider("baiduTranslate")).toEqual([
      "appId",
      "appKey",
    ]);
  });

  it("clears every write-only field before a confirmed deletion", () => {
    const draft = emptyCredentialDraft();
    draft.apiKey = "new-api-secret";
    draft.appId = "123456";
    draft.secretId = "new-secret-id";
    draft.secretKey = "new-secret-key";

    const editing = { draft, editingSavedCredential: true };
    expect(credentialEditorStateAfterDeleteRequest(editing, false)).toBe(
      editing,
    );
    expect(credentialEditorStateAfterDeleteRequest(editing, true)).toEqual({
      draft: emptyCredentialDraft(),
      editingSavedCredential: false,
    });
  });
});
