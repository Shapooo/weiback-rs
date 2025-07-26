<template>
  <div class="container">
    <h2>用户备份</h2>
    <div class="input-group">
      <label for="user-id">用户ID：</label>
      <input type="text" v-model="userId" placeholder="请输入用户ID" id="user-id" />
    </div>
    <div class="input-group">
      <label for="start-page">备份范围：</label>
      <input type="number" v-model="startPage" placeholder="起始页" id="start-page" />
      <span>-</span>
      <input type="number" v-model="endPage" placeholder="结束页" id="end-page" />
    </div>
    <button @click="startBackup">开始备份</button>
    <p v-if="message">{{ message }}</p>
  </div>
</template>

<script setup lang="ts">
import { ref } from 'vue';
import { invoke } from '@tauri-apps/api/core';

const userId = ref('');
const startPage = ref<number | null>(null);
const endPage = ref<number | null>(null);
const message = ref('');

async function startBackup() {
  if (!userId.value) {
    message.value = '请输入用户ID。';
    return;
  }
  if (startPage.value === null || endPage.value === null) {
    message.value = '请输入完整的备份范围。';
    return;
  }
  if (startPage.value <= 0 || endPage.value <= 0) {
    message.value = '页码必须是正数。';
    return;
  }
  if (startPage.value > endPage.value) {
    message.value = '起始页不能大于结束页。';
    return;
  }

  message.value = '正在开始备份，请稍候...';
  try {
    await invoke('backup_user', {
      uid: userId.value,
      range: [startPage.value, endPage.value],
    });
    message.value = '用户备份任务已成功启动。';
  } catch (e) {
    message.value = `备份失败: ${e}`;
    console.error(e);
  }
}
</script>

<style scoped>
.container {
  padding: 20px;
  display: flex;
  flex-direction: column;
  gap: 20px;
  align-items: flex-start;
}
.input-group {
  display: flex;
  align-items: center;
  gap: 10px;
}
input[type="text"] {
    width: 200px;
    padding: 8px;
}
input[type="number"] {
  width: 100px;
  padding: 8px;
}
button {
  padding: 10px 20px;
  cursor: pointer;
}
</style>
