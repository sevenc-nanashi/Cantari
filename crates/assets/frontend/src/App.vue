<script setup lang="ts">
import { ref } from "vue";
import PageHeader from "./components/PageHeader.vue";
import PageFooter from "./components/PageFooter.vue";
import PathsTable from "./components/PathsTable.vue";
import { useOngens, useSettings } from "./composables/useData.ts";
import { ElLoading, ElMessage } from "element-plus";

const settings = useSettings();

const newPaths = ref<string[]>([]);
const deletePaths = ref<string[]>([]);
const newPathsInput = ref("");

const ongens = ref(structuredClone(useOngens()));

const ongenSettings = ref(structuredClone(settings.ongen_settings));

const ongenLimit = ref(settings.ongen_limit);

const addPath = () => {
  newPaths.value.push(newPathsInput.value.trim());
  newPaths.value = Array.from(new Set(newPaths.value));
  newPathsInput.value = "";
};

const applying = ref(false);
const submit = async () => {
  const paths = settings.paths
    .concat(newPaths.value)
    .filter((path) => !deletePaths.value.includes(path));
  applying.value = true;
  const loading = ElLoading.service({
    lock: true,
    text: "反映中...",
  });
  const res = await fetch("/settings", {
    method: "PUT",
    headers: {
      "Content-Type": "application/json",
    },
    body: JSON.stringify({
      paths,
      ongenLimit: ongenLimit.value,
      ongenSettings: ongenSettings.value,
    }),
  });
  if (res.ok) {
    location.reload();
  } else {
    loading.close();
    applying.value = false;
    ElMessage.error("エラーが発生しました。");
  }
};

const reset = () => {
  location.reload();
};
</script>

<template>
  <PageHeader />
  <ElDivider />
  <section>
    <h2>音源のパス</h2>
    <p>
      音源のパスを指定します。character.txtの入っているフォルダ、またはその1つ上を指定して下さい。
    </p>
    <PathsTable v-model:deletePaths="deletePaths" :newPaths="newPaths" />
    <div class="add-path">
      <ElInput
        v-model="newPathsInput"
        placeholder="C:/Users/Nanatsuki/AppData/Roaming/UTAU/voice"
      />
      <ElButton @click="addPath">追加</ElButton>
    </div>
  </section>
  <section>
    <h2>音源数上限</h2>
    <p>
      読み込む音源の上限を指定します。この数を超える音源は読み込まれません。
      0を指定すると上限を無効にしますが、多すぎるとデータ準備が終わらなくなる可能性があります。
    </p>
    <ElInputNumber v-model="ongenLimit" :min="0" />
  </section>
  <section>
    <h2>音源設定</h2>
    <OngenSettings
      v-if="Object.keys(ongens).length > 0"
      v-model:ongenSettings="ongenSettings"
      :ongens
    />
    <p v-else>音源がありません。</p>
  </section>
  <ElDivider />
  <p>
    変更をVoicevoxに反映するには、このボタンを押した後にVoicevoxを再起動する必要があります。
  </p>
  <ElButton
    type="primary"
    @click="submit"
    :disabled="applying"
    native-type="submit"
    >反映</ElButton
  >
  <ElButton
    type="danger"
    class="reset"
    plain
    @click="reset"
    :disabled="applying"
    native-type="reset"
    >反映せずリセット</ElButton
  >
  <PageFooter />
</template>

<style scoped>
.add-path {
  display: flex;
  gap: 0.5em;
}

p {
  margin: 0;
  padding: 0;
}

.hider {
  position: fixed;
  top: 0;
  left: 0;
  width: 100%;
  height: 100%;
  z-index: 1000;

  background-color: #fff8;
  opacity: 0;
  pointer-events: none;
  transition: opacity 0.3s;

  &[data-show="true"] {
    opacity: 1;
    pointer-events: auto;
    cursor: wait;
  }
}
.reset {
  margin: 0;
}
</style>
