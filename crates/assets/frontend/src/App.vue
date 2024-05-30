<script setup lang="ts">
import { ref } from "vue";
import PageHeader from "./components/PageHeader.vue";
import PageFooter from "./components/PageFooter.vue";
import PathsTable from "./components/PathsTable.vue";
import { useSettings } from "./composables/useSettings.ts";
import { ElMessage } from "element-plus";

const settings = useSettings();

const newPaths = ref<string[]>([]);
const deletePaths = ref<string[]>([]);
const newPathsInput = ref("");

const addPath = () => {
  newPaths.value.push(newPathsInput.value.trim());
  newPaths.value = Array.from(new Set(newPaths.value));
  newPathsInput.value = "";
};

const hasSent = ref(false);
const submit = async () => {
  const paths = settings.paths
    .concat(newPaths.value)
    .filter((path) => !deletePaths.value.includes(path));
  hasSent.value = true;
  ElMessage("反映中...");
  await fetch("/settings", {
    method: "PUT",
    headers: {
      "Content-Type": "application/json",
    },
    body: JSON.stringify({
      paths,
    }),
  });
  location.reload();
};
</script>

<template>
  <PageHeader />
  <ElDivider />
  <div>
    <p>
      音源のパスを指定します。character.txtの入っているフォルダ、またはその1つ上を指定して下さい。
    </p>
  </div>
  <PathsTable v-model:deletePaths="deletePaths" :newPaths="newPaths" />
  <div class="add-paths">
    <ElInput
      v-model="newPathsInput"
      placeholder="C:/Users/Nanatsuki/AppData/Roaming/UTAU/voice"
    />
    <ElButton @click="addPath">追加</ElButton>
  </div>
  <ElDivider />
  <p>
    変更をVoicevoxに反映するには、このボタンを押した後にVoicevoxを再起動する必要があります。
  </p>
  <ElButton type="primary" @click="submit" :disabled="hasSent">反映</ElButton>
  <PageFooter />
</template>

<style scoped>
.add-paths {
  display: flex;
  gap: 0.5rem;
}

p {
  margin: 0;
  padding: 0;
}
</style>
