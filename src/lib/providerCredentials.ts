import type {
  ProviderCredentialsInput,
  ServiceProvider,
} from "./types";

export type CredentialFieldName =
  | "apiKey"
  | "endpoint"
  | "deployment"
  | "transcriptionDeployment"
  | "appId"
  | "secretId"
  | "secretKey"
  | "appKey";

export type CredentialDraft = Record<CredentialFieldName, string>;

interface CredentialEditorLocalState {
  draft: CredentialDraft;
  editingSavedCredential: boolean;
}

export function emptyCredentialDraft(): CredentialDraft {
  return {
    apiKey: "",
    endpoint: "",
    deployment: "",
    transcriptionDeployment: "",
    appId: "",
    secretId: "",
    secretKey: "",
    appKey: "",
  };
}

/** Clears write-only fields before a confirmed credential deletion starts. */
export function credentialEditorStateAfterDeleteRequest(
  current: CredentialEditorLocalState,
  confirmingDelete: boolean,
): CredentialEditorLocalState {
  if (!confirmingDelete) return current;
  return {
    draft: emptyCredentialDraft(),
    editingSavedCredential: false,
  };
}

export function credentialFieldsForProvider(
  provider: ServiceProvider,
): readonly CredentialFieldName[] {
  switch (provider) {
    case "azureOpenAIRealtime":
      return [
        "endpoint",
        "deployment",
        "transcriptionDeployment",
        "apiKey",
      ];
    case "tencentCloud":
      return ["appId", "secretId", "secretKey"];
    case "baiduTranslate":
      return ["appId", "appKey"];
    default:
      return ["apiKey"];
  }
}

export function buildProviderCredentials(
  provider: ServiceProvider,
  draft: CredentialDraft,
): ProviderCredentialsInput | null {
  const values = Object.fromEntries(
    Object.entries(draft).map(([key, value]) => [key, value.trim()]),
  ) as CredentialDraft;
  if (
    credentialFieldsForProvider(provider).some((field) => !values[field])
  ) {
    return null;
  }

  switch (provider) {
    case "azureOpenAIRealtime":
      return {
        kind: "azureOpenAI",
        endpoint: values.endpoint,
        deployment: values.deployment,
        transcriptionDeployment: values.transcriptionDeployment,
        apiKey: values.apiKey,
      };
    case "tencentCloud":
      return {
        kind: "tencentCloud",
        appId: values.appId,
        secretId: values.secretId,
        secretKey: values.secretKey,
      };
    case "baiduTranslate":
      return {
        kind: "baiduTranslate",
        appId: values.appId,
        appKey: values.appKey,
      };
    default:
      return { kind: "apiKey", apiKey: values.apiKey };
  }
}
