<script setup lang="ts">
import { useI18n } from "@vasakgroup/tauri-plugin-i18n";
import { displayName, useGreeter } from "@/composables/useGreeter";
import type { SystemUser } from "@/types/greeter";

const { t } = useI18n();
const { users, selectedUser, usingManualEntry, selectUser, useManualEntry } =
  useGreeter();

/** Initial used when the account has no picture. */
const initial = (user: SystemUser) =>
  displayName(user).charAt(0).toUpperCase();
</script>

<template>
  <div class="flex flex-col gap-2 w-full">
    <h3 class="text-sm font-semibold text-primary mb-2 uppercase">
      {{ t("login.selectUser") }}
    </h3>

    <p v-if="users.length === 0" class="text-tx-muted text-sm">
      {{ t("login.noUsers") }}
    </p>

    <button
      v-for="user in users"
      :key="user.uid"
      type="button"
      @click="selectUser(user)"
      class="p-3 border rounded-corner cursor-pointer hover:bg-ui-surface transition-colors flex items-center gap-4 text-left"
      :class="
        !usingManualEntry && selectedUser?.uid === user.uid
          ? 'bg-secondary/30 border-primary ring-1 ring-secondary'
          : 'border-ui-border'
      "
    >
      <img
        v-if="user.avatar"
        :src="user.avatar"
        alt=""
        class="w-10 h-10 rounded-full object-cover shrink-0"
      />
      <div
        v-else
        class="w-10 h-10 bg-primary rounded-full flex items-center justify-center text-tx-on-primary font-bold shrink-0"
      >
        {{ initial(user) }}
      </div>

      <div class="min-w-0">
        <div class="font-bold text-tx-main truncate">
          {{ displayName(user) }}
        </div>
        <div class="text-xs text-tx-muted truncate">@{{ user.name }}</div>
      </div>
    </button>

    <!-- Always available: an account can exist without being enumerable
         (LDAP without enumeration, a hidden administrator), and with no users
         at all this is the only way in. -->
    <button
      type="button"
      @click="useManualEntry()"
      class="p-3 border rounded-corner cursor-pointer hover:bg-ui-surface transition-colors flex items-center gap-4 text-left"
      :class="
        usingManualEntry
          ? 'bg-secondary/30 border-primary ring-1 ring-secondary'
          : 'border-ui-border'
      "
    >
      <div
        class="w-10 h-10 rounded-full border border-dashed border-ui-border flex items-center justify-center text-tx-muted shrink-0"
      >
        ?
      </div>
      <div class="font-bold text-tx-main">{{ t("login.otherUser") }}</div>
    </button>
  </div>
</template>
