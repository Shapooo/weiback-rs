<template>
  <div>
    <nav class="tabs">
      <router-link to="/online-backup/user" class="tab-link">用户备份</router-link>
      <router-link to="/online-backup/favorites" class="tab-link">收藏备份</router-link>
    </nav>
    <router-view></router-view>
  </div>
</template>

<script setup lang="ts">
// No script needed for this layout component
</script>

<style scoped>
.tabs {
  display: flex;
  border-bottom: 1px solid #ccc;
  padding: 0 20px;
}
.tab-link {
  padding: 10px 20px;
  cursor: pointer;
  text-decoration: none;
  color: #333;
  border-bottom: 2px solid transparent;
}
.tab-link:hover {
  background-color: #f0f0f0;
}
.router-link-active {
  border-bottom-color: #007bff;
  font-weight: bold;
  color: #007bff;
}
</style>
