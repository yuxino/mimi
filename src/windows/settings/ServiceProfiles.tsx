import { useMemo, useState } from "react";
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
import { InlineFeedback, SettingsSection } from "./SettingsPrimitives";

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

  const initialProfile =
    settings.profiles.find(
      (profile) => profile.id === settings.activeProfileId,
    ) ?? settings.profiles[0];
  const [selectedProfileId, setSelectedProfileId] = useState(
    initialProfile?.id ?? settings.activeProfileId,
  );
  const [showsProviderPicker, setShowsProviderPicker] = useState(false);
  const [nameDraft, setNameDraft] = useState(initialProfile?.name ?? "");
  const [apiKey, setApiKey] = useState("");
  const [pendingAction, setPendingAction] = useState<PendingAction>(null);
  const [feedback, setFeedback] = useState<Feedback | null>(null);
  const [renderedProfile, setRenderedProfile] = useState({
    id: initialProfile?.id,
    name: initialProfile?.name,
  });

  const selectedProfile = useMemo(
    () =>
      settings.profiles.find((profile) => profile.id === selectedProfileId) ??
      settings.profiles.find(
        (profile) => profile.id === settings.activeProfileId,
      ) ??
      settings.profiles[0],
    [selectedProfileId, settings.activeProfileId, settings.profiles],
  );

  // Keep local, write-only editor drafts aligned with a profile selected or
  // renamed through a settings snapshot. This is React's render-time derived
  // state adjustment, avoiding an effect-driven extra commit.
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
    setApiKey("");
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

  const handleSelect = async () => {
    if (!selectedProfile || selectedProfile.id === settings.activeProfileId) {
      return;
    }
    await perform(
      "select",
      () => selectProfile(selectedProfile.id),
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

  const handleSaveCredential = async () => {
    if (!selectedProfile || !apiKey.trim()) return;
    const replacement = apiKey.trim();
    // The editor is write-only: discard the plaintext draft before the
    // Keychain operation starts, and never restore it after a failure.
    setApiKey("");
    await perform(
      "save-key",
      () => saveProfileAPIKey(selectedProfile.id, replacement),
      I18N.settings.credentialsSaved,
    );
  };

  const handleDeleteCredential = async () => {
    if (
      !selectedProfile ||
      !window.confirm(I18N.settings.deleteCredentialsConfirm)
    ) {
      return;
    }
    const snapshot = await perform(
      "delete-key",
      () => deleteProfileAPIKey(selectedProfile.id),
      I18N.settings.credentialsDeleted,
    );
    if (snapshot) setApiKey("");
  };

  return (
    <SettingsSection
      id="service-profiles"
      icon="key"
      title={I18N.settings.serviceProfilesTitle}
      description={I18N.settings.serviceProfilesDescription}
      action={
        <button
          type="button"
          className="settings-button settings-button--compact settings-button--quiet"
          disabled={mutationsDisabled || atProfileLimit}
          onClick={() => setShowsProviderPicker((visible) => !visible)}
        >
          <Icon name="plus" />
          {I18N.settings.addProfile}
        </button>
      }
    >
      {sessionIsActive && (
        <InlineFeedback tone="info" icon="lock">
          {I18N.settings.profileMutationsLocked}
        </InlineFeedback>
      )}

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
        <div className="profile-list" aria-label={I18N.settings.serviceProfilesTitle}>
          <span className="profile-list__count">
            {I18N.settings.profileCount(settings.profiles.length)}
          </span>
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
                <label htmlFor="profile-name">{I18N.settings.profileName}</label>
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

              <div className="profile-provider-summary">
                <span>{I18N.settings.profileProvider}</span>
                <strong>{providerDisplayName(selectedProfile.provider)}</strong>
                <small>{providerDescription(selectedProfile.provider)}</small>
              </div>
            </form>

            <div className="credential-panel">
              <div className="credential-panel__heading">
                <span>
                  <span className="credential-panel__label">
                    {I18N.settings.credentials}
                  </span>
                  <CredentialBadge state={selectedProfile.credentialState} />
                </span>
                {selectedProfile.credentialState === "present" && (
                  <button
                    type="button"
                    className="settings-link settings-link--danger"
                    disabled={mutationsDisabled}
                    onClick={() => void handleDeleteCredential()}
                  >
                    {I18N.settings.deleteCredentials}
                  </button>
                )}
              </div>

              {selectedProfile.credentialState === "unavailable" && (
                <p className="credential-unavailable" role="status">
                  {I18N.settings.credentialUnavailableHelp}
                </p>
              )}

              <form
                className="credential-form"
                onSubmit={(event) => {
                  event.preventDefault();
                  void handleSaveCredential();
                }}
              >
                <label className="settings-field">
                  <span>{I18N.settings.apiKey}</span>
                  <input
                    id="profile-api-key"
                    type="password"
                    value={apiKey}
                    autoComplete="new-password"
                    spellCheck={false}
                    aria-describedby="credential-storage-note"
                    disabled={mutationsDisabled}
                    placeholder={I18N.settings.apiKeyPlaceholder}
                    onChange={(event) => {
                      setApiKey(event.target.value);
                      setFeedback(null);
                    }}
                  />
                </label>
                <p id="credential-storage-note" className="settings-caption">
                  <Icon name="shield-check" />
                  <span>{I18N.settings.credentialNote}</span>
                </p>
                <button
                  type="submit"
                  className="settings-button settings-button--primary"
                  disabled={mutationsDisabled || !apiKey.trim()}
                >
                  {selectedProfile.credentialState === "present"
                    ? I18N.settings.replaceCredentials
                    : I18N.settings.saveCredentials}
                </button>
              </form>
            </div>

            {feedback && (
              <InlineFeedback tone={feedback.tone}>
                {feedback.message}
              </InlineFeedback>
            )}

            <div className="profile-editor__footer">
              <button
                type="button"
                className="settings-button settings-button--quiet"
                disabled={
                  mutationsDisabled ||
                  selectedProfile.id === settings.activeProfileId
                }
                onClick={() => void handleSelect()}
              >
                <Icon name="checkmark-circle" />
                {I18N.settings.useProfile}
              </button>
              <button
                type="button"
                className="settings-link settings-link--danger"
                disabled={mutationsDisabled || settings.profiles.length <= 1}
                onClick={() => void handleDelete()}
              >
                <Icon name="trash" />
                {I18N.settings.deleteProfile}
              </button>
            </div>
          </div>
        )}
      </div>
    </SettingsSection>
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
