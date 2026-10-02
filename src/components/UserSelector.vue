<script setup lang="ts">
/**
 * Las cuentas del saludo: el `OptionGroup` de la librería en forma de tarjeta,
 * con el avatar de cada una. Es un grupo de opciones de verdad (`radiogroup`):
 * las flechas pasan de una cuenta a otra y el lector dice cuál está elegida.
 */
import { useI18n } from "@vasakgroup/tauri-plugin-i18n";
import { OptionGroup, type OptionGroupOption } from "@vasakgroup/vue-libvasak";
import { computed } from "vue";
import { displayName, useGreeter } from "@/composables/useGreeter";

const { t } = useI18n();
const { users, selectedUser, usingManualEntry, selectUser, useManualEntry } =
  useGreeter();

/** El valor de «Otra cuenta…»: ningún nombre de cuenta puede empezar con «:». */
const MANUAL = ":manual";

const options = computed<OptionGroupOption<string>[]>(() => [
  ...users.value.map((user) => ({
    value: user.name,
    label: displayName(user),
    description: `@${user.name}`,
    avatar: user.avatar,
    avatarName: displayName(user),
  })),
  // Always available: an account can exist without being enumerable (LDAP
  // without enumeration, a hidden administrator), and with no users at all
  // this is the only way in.
  { value: MANUAL, label: t("login.otherUser"), icon: "avatar-default" },
]);

const chosen = computed<string | null>({
  get: () =>
    usingManualEntry.value ? MANUAL : (selectedUser.value?.name ?? null),
  set: (value) => {
    if (value === MANUAL) {
      useManualEntry();
      return;
    }
    const user = users.value.find((candidate) => candidate.name === value);
    if (user) selectUser(user);
  },
});
</script>

<template>
  <div class="flex w-full min-w-0 flex-col gap-2">
    <!-- Atenuado y no en el primario: el acento es para lo que actúa, y el
         primario de fábrica sobre la tarjeta clara no llega a 4,5:1. -->
    <h3 class="mb-2 text-label-xs font-semibold uppercase tracking-wider text-tx-muted">
      {{ t("login.selectUser") }}
    </h3>

    <p v-if="users.length === 0" class="text-body-s text-tx-muted">
      {{ t("login.noUsers") }}
    </p>

    <OptionGroup
      v-model="chosen"
      :options="options"
      :label="t('login.selectUser')"
      variant="card"
    />
  </div>
</template>
