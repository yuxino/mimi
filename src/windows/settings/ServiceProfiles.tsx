import { useMemo, useState, type FormEvent } from "react";
import { Icon } from "../../components/Icon";
import { I18N, providerDisplayName } from "../../lib/i18n";
import { subtitlePreferencesChanged } from "../../lib/providerCapabilities";
import { useStore } from "../../lib/store";
import type {
  CredentialState,
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
  | "create"
  | "rename"
  | "select"
  | "delete"
  | "save-key"
  | "delete-key"
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
  const saveProfileAPIKey = useStore((state) => state.saveProfileAPIKey);
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
    const name =
      provider === "alibabaCloud"
        ? I18N.settings.defaultAlibabaProfileName
        : I18N.settings.defaultOpenAIProfileName;
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

  const handleDelete = async () => {
    if (
      !selectedProfile ||
      settings.profiles.length <= 1 ||
      !window.confirm(I18N.settings.deleteProfileConfirm(selectedProfile.name))
    ) {
      return;
    }
    const snapshot = await perform(
      "delete",
      () => deleteProfile(selectedProfile.id),
      I18N.settings.profileDeleted,
    );
    if (snapshot) setSelectedProfileId(snapshot.activeProfileId);
  };

  const handleSaveCredential = async (
    profileId: string,
    replacement: string,
  ) => {
    return perform(
      "save-key",
      () => saveProfileAPIKey(profileId, replacement),
      I18N.settings.credentialsSaved,
    );
  };

  const handleDeleteCredential = async (profileId: string) => {
    if (!window.confirm(I18N.settings.deleteCredentialsConfirm)) return;
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
            onDelete={() => handleDeleteCredential(activeProfile.id)}
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
            <small>{I18N.settings.profileCount(settings.profiles.length)}</small>
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
                  onSelect={() => setSelectedProfileId(profile.id)}
                />
              ))}
            </div>

            {selectedProfile && (
              <div className="profile-editor">
                <div className="profile-editor__identity">
                  <ProviderMark provider={selectedProfile.provider} />
                  <span>
                    <strong>{selectedProfile.name}</strong>
                    <small>{providerDisplayName(selectedProfile.provider)}</small>
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
                    onDelete={() =>
                      handleDeleteCredential(selectedProfile.id)
                    }
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
                    onClick={() => void handleDelete()}
                  >
                    <Icon name="trash" />
                    {I18N.settings.deleteProfile}
                  </button>
                </div>
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
  onDelete,
}: {
  profile: ServiceProfile;
  inputId: string;
  disabled: boolean;
  busy: boolean;
  onSave: (replacement: string) => Promise<unknown>;
  onDelete: () => Promise<unknown>;
}) {
  const [apiKey, setApiKey] = useState("");
  const [editingSavedCredential, setEditingSavedCredential] = useState(false);
  const noteId = `${inputId}-storage-note`;

  const handleSubmit = (event: FormEvent<HTMLFormElement>) => {
    event.preventDefault();
    const replacement = apiKey.trim();
    if (!replacement) return;
    // Discard plaintext before secure storage I/O and never restore it after a
    // failure. The editor remains a write-only credential surface.
    setApiKey("");
    void onSave(replacement).then((saved) => {
      if (saved) setEditingSavedCredential(false);
    });
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
            disabled={disabled}
            onClick={() => void onDelete()}
          >
            {I18N.settings.deleteCredentials}
          </button>
        </span>
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
            disabled={disabled}
            onClick={() => void onDelete()}
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

      <form className="credential-form" onSubmit={handleSubmit}>
        <label className="settings-field" htmlFor={inputId}>
          <span>{I18N.settings.apiKey}</span>
          <input
            id={inputId}
            type="password"
            value={apiKey}
            autoComplete="new-password"
            spellCheck={false}
            aria-describedby={noteId}
            disabled={disabled}
            placeholder={I18N.settings.apiKeyPlaceholder}
            onChange={(event) => setApiKey(event.target.value)}
          />
        </label>
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
                setApiKey("");
                setEditingSavedCredential(false);
              }}
            >
              {I18N.settings.cancel}
            </button>
          )}
          <button
            type="submit"
            className="settings-button settings-button--primary"
            disabled={disabled || !apiKey.trim()}
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
        {(["alibabaCloud", "openAIRealtime"] as const).map((provider) => (
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
      className="profile-list-item"
      data-selected={selected}
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
      <Icon name={provider === "alibabaCloud" ? "cloud" : "waves"} />
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
  return provider === "alibabaCloud"
    ? I18N.settings.providerAlibabaDescription
    : I18N.settings.providerOpenAIDescription;
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
