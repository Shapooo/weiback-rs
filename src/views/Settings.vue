<template>
  <n-card class="box-card" title="设置">
    <n-form label-placement="left" label-width="auto">
      <n-form-item label="暗色模式">
        <n-switch :value="isDarkMode" @update:value="toggleDarkMode" />
      </n-form-item>
    </n-form>
  </n-card>
</template>

<script setup lang="ts">
import { isDarkMode, toggleDarkMode } from '../store/theme';
</script>

<style scoped>
.box-card {
  max-width: 500px;
  margin: 20px auto;
}
</style>
