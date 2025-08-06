<template>
  <n-card class="box-card" title="用户备份">
    <n-form :model="form" label-placement="left" label-width="auto">
      <n-form-item label="用户ID">
        <n-input v-model:value="form.userId" placeholder="请输入用户ID" />
      </n-form-item>
      <n-form-item label="备份范围">
        <n-grid :cols="24" :x-gap="8">
          <n-gi :span="11">
            <n-input-number v-model:value="form.startPage" :min="1" :step="20" placeholder="起始页" style="width: 100%;" />
          </n-gi>
          <n-gi :span="2" style="text-align: center;">-</n-gi>
          <n-gi :span="11">
            <n-input-number v-model:value="form.endPage" :min="20" :step="20" placeholder="结束页" style="width: 100%;" />
          </n-gi>
        </n-grid>
      </n-form-item>
      <n-form-item>
        <n-button type="primary" tertiary @click="startBackup">开始备份</n-button>
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
  userId: '',
  startPage: 1,
  endPage: 10,
});

async function startBackup() {
  if (!form.userId) {
    message.error('请输入用户ID');
    return;
  }
  if (form.startPage > form.endPage) {
    message.error('起始页不能大于结束页');
    return;
  }

  message.info('正在开始备份，请稍候...');
  try {
    await invoke('backup_user', {
      uid: form.userId,
      range: [form.startPage, form.endPage],
    });
    message.success('用户备份任务已成功启动');
  } catch (e) {
    message.error(`备份失败: ${e}`);
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
