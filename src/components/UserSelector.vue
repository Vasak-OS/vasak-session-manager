<script setup lang="ts">
// TODO(2.4.0): el selector de usuario pasa a la librería (vue-libvasak 2.4.0).
// Hasta entonces se arma acá con `ListRow` y `Avatar`, que sí están en la 2.3.
import { useI18n } from "@vasakgroup/tauri-plugin-i18n";
import { Avatar, ListRow } from "@vasakgroup/vue-libvasak";
import { displayName, useGreeter } from "@/composables/useGreeter";

const { t } = useI18n();
const { users, selectedUser, usingManualEntry, selectUser, useManualEntry } =
  useGreeter();
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

    <!-- `!size-10`: el avatar mide 40 como antes; la 2.3 salta de 32 a 48
         (pedido para la 2.4.0). El nombre ya está escrito al lado, así que el
         avatar no lo repite (`alt=""`). -->
    <ListRow
      v-for="user in users"
      :key="user.uid"
      role="button"
      :selected="!usingManualEntry && selectedUser?.uid === user.uid"
      :title="displayName(user)"
      :description="`@${user.name}`"
      @click="selectUser(user)"
    >
      <template #leading>
        <Avatar
          :src="user.avatar"
          :name="displayName(user)"
          alt=""
          size="lg"
          class="!size-10"
        />
      </template>
    </ListRow>

    <!-- Always available: an account can exist without being enumerable
         (LDAP without enumeration, a hidden administrator), and with no users
         at all this is the only way in. Sin nombre, el avatar dibuja el icono
         de persona del tema en vez del «?» de antes. -->
    <ListRow
      role="button"
      :selected="usingManualEntry"
      :title="t('login.otherUser')"
      @click="useManualEntry()"
    >
      <template #leading>
        <Avatar alt="" size="lg" class="!size-10" />
      </template>
    </ListRow>
  </div>
</template>
