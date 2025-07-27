<template>
  <n-card class="box-card" title="本地导出">
    <n-form :model="form" label-placement="left" label-width="auto">
      <n-form-item label="导出范围">
        <n-grid :cols="24" :x-gap="8">
          <n-gi :span="11">
            <n-input-number v-model:value="form.startPage" :min="1" placeholder="起始页" style="width: 100%;" />
          </n-gi>
          <n-gi :span="2" style="text-align: center;">-</n-gi>
          <n-gi :span="11">
            <n-input-number v-model:value="form.endPage" :min="1" placeholder="结束页" style="width: 100%;" />
          </n-gi>
        </n-grid>
      </n-form-item>
      <n-form-item>
        <n-button type="primary" tertiary @click="startExport">开始导出</n-button>
      </n-form-item>
    </n-form>
  </n-card>
</template>

<script setup lang="ts">
import { reactive } from 'vue';
import { invoke } from '@tauri-apps/api/core';
import { useMessage } from 'naive-ui';

const message = useMessage();
const form = reactive({
  startPage: 1,
  endPage: 10,
});

async function startExport() {
  if (form.startPage > form.endPage) {
    message.error('起始页不能大于结束页');
    return;
  }

  message.info('正在开始导出，请稍候...');
  try {
    await invoke('export_from_local', {
      range: [form.startPage, form.endPage],
    });
    message.success('本地导出任务已成功启动');
  } catch (e) {
    message.error(`导出失败: ${e}`);
    console.error(e);
  }
}
</script>

<style scoped>
.box-card {
  max-width: 500px;
  margin: 20px auto;
}
</style>
