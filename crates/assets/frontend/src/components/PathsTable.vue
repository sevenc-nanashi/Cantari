<script setup lang="ts">
import { useSettings } from "../composables/useData.ts";
import { computed } from "vue";

const props = defineProps<{ newPaths: string[] }>();
const deletePaths = defineModel<string[]>("deletePaths", { default: [] });

const settings = useSettings();
const tableData = computed(() =>
  [...settings.paths, ...props.newPaths].map((path) => ({ path })),
);

const rowClassName = ({ row }: { row: { path: string } }) => {
  if (deletePaths.value.includes(row.path)) {
    return "delete-row";
  }

  if (props.newPaths.includes(row.path)) {
    return "new-row";
  }

  return "";
};
const deleteRow = (index: number) => {
  deletePaths.value = deletePaths.value.concat(tableData.value[index].path);
};
const undeleteRow = (index: number) => {
  deletePaths.value = deletePaths.value.filter(
    (path) => path !== tableData.value[index].path,
  );
};
</script>

<template>
  <ElTable
    :data="tableData"
    style="width: 100%"
    max-height="250"
    :row-class-name="rowClassName"
  >
    <ElTableColumn fixed prop="path" label="パス" />
    <ElTableColumn fixed="right" label="操作" width="120">
      <template #default="scope">
        <ElButton
          v-if="!deletePaths.includes(scope.row.path)"
          link
          type="danger"
          size="small"
          @click.prevent="deleteRow(scope.$index)"
        >
          削除
        </ElButton>
        <ElButton
          v-else
          link
          type="info"
          size="small"
          @click.prevent="undeleteRow(scope.$index)"
        >
          キャンセル
        </ElButton>
      </template>
    </ElTableColumn>
  </ElTable>
</template>

<style scoped>
header {
  font-size: 2em;
  margin-bottom: 20px;
  border-bottom: 1px solid #ccc;
  padding-bottom: 10px;
}

:deep(.delete-row) {
  background-color: #ffe1e1;
}

:deep(.new-row) {
  background-color: #e1f7d5;
}
</style>
