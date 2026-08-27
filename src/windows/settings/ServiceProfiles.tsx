import { useEffect, useMemo, useRef, useState, type FormEvent } from "react";
import { Icon } from "../../components/Icon";
import { I18N, providerDisplayName } from "../../lib/i18n";
import {
  SERVICE_PROVIDERS,
  subtitlePreferencesChanged,
} from "../../lib/providerCapabilities";
import {
  buildProviderCredentials,
  credentialEditorStateAfterDeleteRequest,
  credentialFieldsForProvider,
  emptyCredentialDraft,
  type CredentialDraft,
  type CredentialFieldName,
} from "../../lib/providerCredentials";
import { useStore } from "../../lib/store";
import type {
  CredentialState,
  ProviderCredentialsInput,
  ServiceProfile,
  ServiceProvider,
  SettingsSnapshot,
} from "../../lib/types";
import {
  InlineFeedback,
  SettingsSection,
  SettingsSelect,
} from "./SettingsPrimitives";

type Feedback = { tone: "success" | "error" | "info"; message: string };
type PendingAction =
  "create" | "rename" | "select" | "delete" | "save-key" | "delete-key" | null;
type PendingConfirmation =
  | { kind: "profile"; profileId: string; name: string }
  | { kind: "credential"; profileId: string }
  | null;

export function ServiceProfiles({
  settings,
  sessionIsActive,
}: {
  settings: SettingsSnapshot;
  sessionIsActive: boolean;
}) {
  const createProfile = useStore((state) => state.createProfile);
  const updateProfile = useStore((state) => state.updateProfile);
  const selectProfile = useStore((state) => state.selectProfile);
  const deleteProfile = useStore((state) => state.deleteProfile);
  const saveProfileCredentials = useStore(
    (state) => state.saveProfileCredentials,
  );
  const deleteProfileAPIKey = useStore((state) => state.deleteProfileAPIKey);

  const activeProfile =
    settings.profiles.find(
      (profile) => profile.id === settings.activeProfileId,
    ) ?? settings.profiles[0];
  const [selectedProfileId, setSelectedProfileId] = useState(
    activeProfile?.id ?? settings.activeProfileId,
  );
  const [showsProviderPicker, setShowsProviderPicker] = useState(false);
  const [nameDraft, setNameDraft] = useState(activeProfile?.name ?? "");
  const [pendingAction, setPendingAction] = useState<PendingAction>(null);
  const [feedback, setFeedback] = useState<Feedback | null>(null);
  const [pendingConfirmation, setPendingConfirmation] =
    useState<PendingConfirmation>(null);
  const [renderedProfile, setRenderedProfile] = useState({
    id: activeProfile?.id,
    name: activeProfile?.name,
  });

  const selectedProfile = useMemo(
    () =>
      settings.profiles.find((profile) => profile.id === selectedProfileId) ??
      activeProfile,
    [activeProfile, selectedProfileId, settings.profiles],
  );

  if (
    selectedProfile &&
    (selectedProfile.id !== renderedProfile.id ||
      selectedProfile.name !== renderedProfile.name)
  ) {
    setRenderedProfile({
      id: selectedProfile.id,
      name: selectedProfile.name,
    });
    setNameDraft(selectedProfile.name);
    setFeedback(null);
    setPendingConfirmation(null);
  }

  const mutationsDisabled = sessionIsActive || pendingAction !== null;
  const atProfileLimit = settings.profiles.length >= 20;

  const perform = async (
    action: Exclude<PendingAction, null>,
    operation: () => Promise<SettingsSnapshot>,
    successFeedback: string | ((snapshot: SettingsSnapshot) => Feedback),
  ): Promise<SettingsSnapshot | null> => {
    setPendingAction(action);
    setFeedback(null);
    try {
      const snapshot = await operation();
      setFeedback(
        typeof successFeedback === "string"
          ? { tone: "success", message: successFeedback }
          : successFeedback(snapshot),
      );
      return snapshot;
    } catch {
      setFeedback({
        tone: "error",
        message: I18N.settings.profileActionFailed,
      });
      return null;
    } finally {
      setPendingAction(null);
    }
  };

  const handleCreate = async (provider: ServiceProvider) => {
    const previousIds = new Set(settings.profiles.map((profile) => profile.id));
    const name = providerDisplayName(provider);
    const snapshot = await perform(
      "create",
      () => createProfile(provider, name),
      I18N.settings.profileCreated,
    );
    if (!snapshot) return;
    const created = snapshot.profiles.find(
      (profile) => !previousIds.has(profile.id),
    );
    if (created) setSelectedProfileId(created.id);
    setShowsProviderPicker(false);
  };

  const handleRename = async () => {
    if (!selectedProfile) return;
    const name = nameDraft.trim();
    if (!name || name === selectedProfile.name) return;
    await perform(
      "rename",
      () => updateProfile(selectedProfile.id, name),
      I18N.settings.profileNameSaved,
    );
  };

  const handleSelect = async (profileId: string) => {
    if (profileId === settings.activeProfileId) return;
    setPendingConfirmation(null);
    setSelectedProfileId(profileId);
    await perform(
      "select",
      () => selectProfile(profileId),
      (snapshot) =>
        subtitlePreferencesChanged(settings, snapshot)
          ? {
              tone: "info",
              message: I18N.settings.profileSelectedWithAdjustments,
            }
          : {
              tone: "success",
              message: I18N.settings.profileSelected,
            },
    );
  };

  const requestProfileDelete = () => {
    if (!selectedProfile || settings.profiles.length <= 1) return;
    setPendingConfirmation({
      kind: "profile",
      profileId: selectedProfile.id,
      name: selectedProfile.name,
    });
  };

  const confirmProfileDelete = async () => {
    if (
      !selectedProfile ||
      settings.profiles.length <= 1 ||
      pendingConfirmation?.kind !== "profile" ||
      pendingConfirmation.profileId !== selectedProfile.id
    )
      return;
    setPendingConfirmation(null);
    const snapshot = await perform(
      "delete",
      () => deleteProfile(selectedProfile.id),
      I18N.settings.profileDeleted,
    );
    if (snapshot) setSelectedProfileId(snapshot.activeProfileId);
  };

  const handleSaveCredential = async (
    profileId: string,
    credentials: ProviderCredentialsInput,
  ) => {
    setPendingConfirmation(null);
    return perform(
      "save-key",
      () => saveProfileCredentials(profileId, credentials),
      I18N.settings.credentialsSaved,
    );
  };

  const requestCredentialDelete = (profileId: string) => {
    setPendingConfirmation({ kind: "credential", profileId });
  };

  const confirmCredentialDelete = async (profileId: string) => {
    if (
      pendingConfirmation?.kind !== "credential" ||
      pendingConfirmation.profileId !== profileId
    )
      return;
    setPendingConfirmation(null);
    await perform(
      "delete-key",
      () => deleteProfileAPIKey(profileId),
      I18N.settings.credentialsDeleted,
    );
  };

  return (
    <SettingsSection
      id="service-profiles"
      title={I18N.settings.serviceProfilesTitle}
    >
      {sessionIsActive && (
        <InlineFeedback tone="info" icon="lock">
          {I18N.settings.profileMutationsLocked}
        </InlineFeedback>
      )}

      {activeProfile && (
        <>
          <div className="active-profile-summary">
            <ProviderMark provider={activeProfile.provider} />
            <span className="active-profile-summary__picker">
              <span>{I18N.settings.currentProfile}</span>
              <SettingsSelect
                value={activeProfile.id}
                disabled={mutationsDisabled || settings.profiles.length === 1}
                label={I18N.settings.currentProfile}
                onChange={(profileId) => void handleSelect(profileId)}
                options={settings.profiles.map((profile) => ({
                  value: profile.id,
                  label: profile.name,
                }))}
              />
            </span>
            <CredentialBadge state={activeProfile.credentialState} />
          </div>

          <CredentialEditor
            key={`active-${activeProfile.id}`}
            profile={activeProfile}
            inputId="profile-api-key"
            disabled={mutationsDisabled}
            busy={
              pendingAction === "save-key" || pendingAction === "delete-key"
            }
            onSave={(replacement) =>
              handleSaveCredential(activeProfile.id, replacement)
            }
            onRequestDelete={() => requestCredentialDelete(activeProfile.id)}
            onConfirmDelete={() =>
              confirmCredentialDelete(activeProfile.id)
            }
            confirmingDelete={
              pendingConfirmation?.kind === "credential" &&
              pendingConfirmation.profileId === activeProfile.id
            }
            onCancelDelete={() => setPendingConfirmation(null)}
          />
        </>
      )}

      {feedback && (
        <InlineFeedback tone={feedback.tone}>{feedback.message}</InlineFeedback>
      )}

      <details className="profile-management">
        <summary>
          <span className="profile-management__summary-copy">
            <strong>{I18N.settings.manageServiceProfiles}</strong>
            <small>
              {I18N.settings.profileCount(settings.profiles.length)}
            </small>
          </span>
          <Icon name="chevron-down" />
        </summary>

        <div className="profile-management__content">
          <div className="profile-management__toolbar">
            <p>{I18N.settings.serviceProfilesDescription}</p>
            <button
              type="button"
              className="settings-button settings-button--compact settings-button--quiet"
              disabled={mutationsDisabled || atProfileLimit}
              onClick={() => setShowsProviderPicker((visible) => !visible)}
            >
              <Icon name="plus" />
              {I18N.settings.addProfile}
            </button>
          </div>

          {atProfileLimit && !sessionIsActive && (
            <InlineFeedback tone="info" icon="exclamation-triangle">
              {I18N.settings.profileLimitReached}
            </InlineFeedback>
          )}

          {showsProviderPicker && !sessionIsActive && (
            <ProviderPicker
              disabled={pendingAction !== null}
              onChoose={(provider) => void handleCreate(provider)}
              onCancel={() => setShowsProviderPicker(false)}
            />
          )}

          <div className="profiles-workspace">
            <div
              className="profile-list"
              aria-label={I18N.settings.serviceProfilesTitle}
            >
              {settings.profiles.map((profile) => (
                <ProfileListItem
                  key={profile.id}
                  profile={profile}
                  selected={profile.id === selectedProfile?.id}
                  active={profile.id === settings.activeProfileId}
                  disabled={pendingAction !== null}
                  onSelect={() => {
                    setSelectedProfileId(profile.id);
                    setPendingConfirmation(null);
                  }}
                />
              ))}
            </div>

            {selectedProfile && (
              <div className="profile-editor">
                <div className="profile-editor__identity">
                  <ProviderMark provider={selectedProfile.provider} />
                  <span>
                    <strong>{selectedProfile.name}</strong>
                    <small>
                      {providerDisplayName(selectedProfile.provider)}
                    </small>
                  </span>
                  {selectedProfile.id === settings.activeProfileId && (
                    <span className="profile-active-badge">
                      <Icon name="checkmark" />
                      {I18N.settings.activeProfile}
                    </span>
                  )}
                </div>

                <form
                  className="profile-form"
                  onSubmit={(event) => {
                    event.preventDefault();
                    void handleRename();
                  }}
                >
                  <div className="settings-field">
                    <label htmlFor="profile-name">
                      {I18N.settings.profileName}
                    </label>
                    <span className="settings-field__inline">
                      <input
                        id="profile-name"
                        value={nameDraft}
                        maxLength={64}
                        disabled={mutationsDisabled}
                        placeholder={I18N.settings.profileNamePlaceholder}
                        onChange={(event) => {
                          setNameDraft(event.target.value);
                          setFeedback(null);
                        }}
                      />
                      <button
                        type="submit"
                        className="settings-button settings-button--quiet"
                        disabled={
                          mutationsDisabled ||
                          !nameDraft.trim() ||
                          nameDraft.trim() === selectedProfile.name
                        }
                      >
                        {I18N.settings.saveName}
                      </button>
                    </span>
                  </div>
                </form>

                {selectedProfile.id !== activeProfile?.id && (
                  <CredentialEditor
                    key={`managed-${selectedProfile.id}`}
                    profile={selectedProfile}
                    inputId={`profile-api-key-${selectedProfile.id}`}
                    disabled={mutationsDisabled}
                    busy={
                      pendingAction === "save-key" ||
                      pendingAction === "delete-key"
                    }
                    onSave={(replacement) =>
                      handleSaveCredential(selectedProfile.id, replacement)
                    }
                    onRequestDelete={() =>
                      requestCredentialDelete(selectedProfile.id)
                    }
                    onConfirmDelete={() =>
                      confirmCredentialDelete(selectedProfile.id)
                    }
                    confirmingDelete={
                      pendingConfirmation?.kind === "credential" &&
                      pendingConfirmation.profileId === selectedProfile.id
                    }
                    onCancelDelete={() => setPendingConfirmation(null)}
                  />
                )}

                <div className="profile-editor__footer">
                  <button
                    type="button"
                    className="settings-button settings-button--quiet"
                    disabled={
                      mutationsDisabled ||
                      selectedProfile.id === settings.activeProfileId
                    }
                    onClick={() => void handleSelect(selectedProfile.id)}
                  >
                    <Icon name="checkmark-circle" />
                    {I18N.settings.useProfile}
                  </button>
                  <button
                    type="button"
                    className="settings-link settings-link--danger"
                    disabled={
                      mutationsDisabled || settings.profiles.length <= 1
                    }
                    onClick={requestProfileDelete}
                  >
                    <Icon name="trash" />
                    {I18N.settings.deleteProfile}
                  </button>
                </div>
                {pendingConfirmation?.kind === "profile" &&
                  pendingConfirmation.profileId === selectedProfile.id && (
                    <DestructiveConfirmation
                      message={I18N.settings.deleteProfileConfirm(
                        pendingConfirmation.name,
                      )}
                      disabled={mutationsDisabled}
                      onCancel={() => setPendingConfirmation(null)}
                      onConfirm={() => void confirmProfileDelete()}
                    />
                  )}
              </div>
            )}
          </div>
        </div>
      </details>
    </SettingsSection>
  );
}

function CredentialEditor({
  profile,
  inputId,
  disabled,
  busy,
  onSave,
  onRequestDelete,
  onConfirmDelete,
  confirmingDelete,
  onCancelDelete,
}: {
  profile: ServiceProfile;
  inputId: string;
  disabled: boolean;
  busy: boolean;
  onSave: (credentials: ProviderCredentialsInput) => Promise<unknown>;
  onRequestDelete: () => void;
  onConfirmDelete: () => Promise<unknown>;
  confirmingDelete: boolean;
  onCancelDelete: () => void;
}) {
  const [draft, setDraft] = useState<CredentialDraft>(emptyCredentialDraft);
  const [editingSavedCredential, setEditingSavedCredential] = useState(false);
  const noteId = `${inputId}-storage-note`;
  const credentials = buildProviderCredentials(profile.provider, draft);

  const handleSubmit = (event: FormEvent<HTMLFormElement>) => {
    event.preventDefault();
    if (!credentials) return;
    // Discard plaintext before secure storage I/O and never restore it after a
    // failure. The editor remains a write-only credential surface.
    setDraft(emptyCredentialDraft());
    void onSave(credentials).then((saved) => {
      if (saved) setEditingSavedCredential(false);
    });
  };

  const handleConfirmDelete = () => {
    const next = credentialEditorStateAfterDeleteRequest(
      { draft, editingSavedCredential },
      true,
    );
    // Clear plaintext before the async keychain deletion starts. A failure
    // must never restore a replacement secret to WebView state or the DOM.
    setDraft(next.draft);
    setEditingSavedCredential(next.editingSavedCredential);
    void onConfirmDelete();
  };

  if (profile.credentialState === "present" && !editingSavedCredential) {
    return (
      <div className="credential-panel credential-panel--saved">
        <span className="credential-panel__saved-actions">
          <button
            type="button"
            className="settings-button settings-button--quiet"
            disabled={disabled}
            onClick={() => setEditingSavedCredential(true)}
          >
            {I18N.settings.replaceCredentials}
          </button>
          <button
            type="button"
            className="settings-link settings-link--danger"
            disabled={disabled || confirmingDelete}
            onClick={onRequestDelete}
          >
            {I18N.settings.deleteCredentials}
          </button>
        </span>
        {confirmingDelete && (
          <DestructiveConfirmation
            message={I18N.settings.deleteCredentialsConfirm}
            disabled={disabled}
            onCancel={onCancelDelete}
            onConfirm={handleConfirmDelete}
          />
        )}
      </div>
    );
  }

  return (
    <div className="credential-panel" aria-busy={busy}>
      <div className="credential-panel__heading">
        <span>
          <span className="credential-panel__label">
            {I18N.settings.credentials}
          </span>
          <CredentialBadge state={profile.credentialState} />
        </span>
        {profile.credentialState === "present" && (
          <button
            type="button"
            className="settings-link settings-link--danger"
            disabled={disabled || confirmingDelete}
            onClick={onRequestDelete}
          >
            {I18N.settings.deleteCredentials}
          </button>
        )}
      </div>

      {profile.credentialState === "unavailable" && (
        <p className="credential-unavailable" role="status">
          {I18N.settings.credentialUnavailableHelp}
        </p>
      )}

      {confirmingDelete && (
        <DestructiveConfirmation
          message={I18N.settings.deleteCredentialsConfirm}
          disabled={disabled}
          onCancel={onCancelDelete}
          onConfirm={handleConfirmDelete}
        />
      )}

      <form className="credential-form" onSubmit={handleSubmit}>
        <div className="credential-form__fields">
          {credentialFieldsForProvider(profile.provider).map((field) => {
            const copy = credentialFieldCopy(field);
            const fieldId = `${inputId}-${field}`;
            return (
              <label className="settings-field" htmlFor={fieldId} key={field}>
                <span>{copy.label}</span>
                <input
                  id={fieldId}
                  type={copy.secret ? "password" : "text"}
                  inputMode={field === "appId" ? "numeric" : undefined}
                  value={draft[field]}
                  autoComplete="new-password"
                  spellCheck={false}
                  aria-describedby={noteId}
                  disabled={disabled}
                  placeholder={copy.placeholder}
                  onChange={(event) =>
                    setDraft((current) => ({
                      ...current,
                      [field]: event.target.value,
                    }))
                  }
                />
              </label>
            );
          })}
        </div>
        <p id={noteId} className="settings-caption">
          <Icon name="shield-check" />
          <span>{I18N.settings.credentialNote}</span>
        </p>
        <span className="credential-form__actions">
          {profile.credentialState === "present" && (
            <button
              type="button"
              className="settings-link"
              disabled={disabled}
              onClick={() => {
                setDraft(emptyCredentialDraft());
                setEditingSavedCredential(false);
              }}
            >
              {I18N.settings.cancel}
            </button>
          )}
          <button
            type="submit"
            className="settings-button settings-button--primary"
            disabled={disabled || !credentials}
          >
            {profile.credentialState === "present"
              ? I18N.settings.replaceCredentials
              : I18N.settings.saveCredentials}
          </button>
        </span>
      </form>
    </div>
  );
}

function credentialFieldCopy(field: CredentialFieldName): {
  label: string;
  placeholder: string;
  secret: boolean;
} {
  switch (field) {
    case "apiKey":
      return {
        label: I18N.settings.apiKey,
        placeholder: I18N.settings.apiKeyPlaceholder,
        secret: true,
      };
    case "endpoint":
      return {
        label: I18N.settings.azureEndpoint,
        placeholder: I18N.settings.azureEndpointPlaceholder,
        secret: false,
      };
    case "deployment":
      return {
        label: I18N.settings.deploymentName,
        placeholder: I18N.settings.deploymentNamePlaceholder,
        secret: false,
      };
    case "transcriptionDeployment":
      return {
        label: I18N.settings.transcriptionDeploymentName,
        placeholder: I18N.settings.transcriptionDeploymentNamePlaceholder,
        secret: false,
      };
    case "appId":
      return {
        label: I18N.settings.appId,
        placeholder: I18N.settings.appIdPlaceholder,
        secret: false,
      };
    case "secretId":
      return {
        label: I18N.settings.secretId,
        placeholder: I18N.settings.secretIdPlaceholder,
        secret: true,
      };
    case "secretKey":
      return {
        label: I18N.settings.secretKey,
        placeholder: I18N.settings.secretKeyPlaceholder,
        secret: true,
      };
    case "appKey":
      return {
        label: I18N.settings.appKey,
        placeholder: I18N.settings.appKeyPlaceholder,
        secret: true,
      };
  }
}

function DestructiveConfirmation({
  message,
  disabled,
  onCancel,
  onConfirm,
}: {
  message: string;
  disabled: boolean;
  onCancel: () => void;
  onConfirm: () => void;
}) {
  const confirmationRef = useRef<HTMLDivElement>(null);

  useEffect(() => {
    const confirmation = confirmationRef.current;
    if (!confirmation) return;
    confirmation.focus({ preventScroll: true });
    confirmation.scrollIntoView({ block: "nearest" });
  }, []);

  return (
    <div
      ref={confirmationRef}
      className="destructive-confirmation"
      role="alert"
      tabIndex={-1}
    >
      <small>{message}</small>
      <span className="destructive-confirmation__actions">
        <button
          type="button"
          className="settings-link"
          disabled={disabled}
          onClick={onCancel}
        >
          {I18N.settings.cancel}
        </button>
        <button
          type="button"
          className="settings-button settings-button--danger settings-button--compact"
          disabled={disabled}
          onClick={onConfirm}
        >
          <Icon name="trash" />
          {I18N.settings.confirmDelete}
        </button>
      </span>
    </div>
  );
}

function ProviderPicker({
  disabled,
  onChoose,
  onCancel,
}: {
  disabled: boolean;
  onChoose: (provider: ServiceProvider) => void;
  onCancel: () => void;
}) {
  return (
    <div className="provider-picker settings-panel">
      <div className="provider-picker__heading">
        <span>
          <strong>{I18N.settings.chooseProvider}</strong>
          <small>{I18N.settings.chooseProviderDescription}</small>
        </span>
        <button
          type="button"
          className="settings-link"
          disabled={disabled}
          onClick={onCancel}
        >
          {I18N.settings.cancel}
        </button>
      </div>
      <div className="provider-picker__options">
        {SERVICE_PROVIDERS.map((provider) => (
          <button
            key={provider}
            type="button"
            className="provider-option"
            disabled={disabled}
            onClick={() => onChoose(provider)}
          >
            <ProviderMark provider={provider} />
            <span>
              <strong>{providerDisplayName(provider)}</strong>
              <small>{providerDescription(provider)}</small>
            </span>
            <Icon name="chevron-down" />
          </button>
        ))}
      </div>
    </div>
  );
}

function ProfileListItem({
  profile,
  selected,
  active,
  disabled,
  onSelect,
}: {
  profile: ServiceProfile;
  selected: boolean;
  active: boolean;
  disabled: boolean;
  onSelect: () => void;
}) {
  return (
    <button
      type="button"
      className={`profile-list-item${selected ? " is-selected" : ""}`}
      aria-pressed={selected}
      aria-current={active ? "true" : undefined}
      disabled={disabled}
      onClick={onSelect}
    >
      <ProviderMark provider={profile.provider} compact />
      <span className="profile-list-item__copy">
        <strong>{profile.name}</strong>
        <small>{providerDisplayName(profile.provider)}</small>
      </span>
      <span
        className="credential-dot"
        data-state={profile.credentialState}
        role="img"
        aria-label={credentialStateText(profile.credentialState)}
        title={credentialStateText(profile.credentialState)}
      />
      {active && <span className="profile-list-item__active" />}
    </button>
  );
}

function ProviderMark({
  provider,
  compact = false,
}: {
  provider: ServiceProvider;
  compact?: boolean;
}) {
  return (
    <span
      className="provider-mark"
      data-provider={provider}
      data-compact={compact}
      aria-hidden="true"
    >
      <Icon
        name={
          provider === "alibabaCloud" || provider === "azureOpenAIRealtime"
            ? "cloud"
            : provider === "xAIRealtime"
              ? "waves"
              : "languages"
        }
      />
    </span>
  );
}

function CredentialBadge({ state }: { state: CredentialState }) {
  return (
    <span className="credential-badge" data-state={state}>
      <span className="credential-dot" data-state={state} />
      {credentialStateText(state)}
    </span>
  );
}

function providerDescription(provider: ServiceProvider): string {
  switch (provider) {
    case "alibabaCloud":
      return I18N.settings.providerAlibabaDescription;
    case "openAIRealtime":
      return I18N.settings.providerOpenAIDescription;
    case "googleGeminiLive":
      return I18N.settings.providerGoogleGeminiDescription;
    case "azureOpenAIRealtime":
      return I18N.settings.providerAzureOpenAIDescription;
    case "volcanoEngine":
      return I18N.settings.providerVolcanoEngineDescription;
    case "tencentCloud":
      return I18N.settings.providerTencentCloudDescription;
    case "baiduTranslate":
      return I18N.settings.providerBaiduTranslateDescription;
    case "xAIRealtime":
      return I18N.settings.providerXAIDescription;
  }
}

function credentialStateText(state: CredentialState): string {
  switch (state) {
    case "present":
      return I18N.settings.credentialPresent;
    case "missing":
      return I18N.settings.credentialMissing;
    case "unavailable":
      return I18N.settings.credentialUnavailable;
  }
}
