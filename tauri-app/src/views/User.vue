<template>
  <n-card class="box-card" title="用户登录">
    <n-form class="login-form">
      <n-form-item v-if="!codeSent">
        <n-input v-model:value="form.phone" placeholder="请输入手机号" size="large" />
      </n-form-item>
      <n-form-item v-if="!codeSent">
        <n-button type="primary" tertiary @click="getVerificationCode" style="width: 100%;" size="large">获取验证码</n-button>
      </n-form-item>

      <template v-if="codeSent">
        <p class="code-prompt">验证码已发送至 {{ form.phone }}</p>
        <n-form-item>
          <div class="code-inputs">
            <n-input
              v-for="(code, index) in form.verificationCode"
              :key="index"
              v-model:value="form.verificationCode[index]"
              :ref="el => setCodeInputRef(el, index)"
              @input="handleCodeInput(index)"
              @keydown="handleKeyDown(index, $event)"
              @focus="handleFocus(index)"
              maxlength="1"
              class="code-input"
              size="large"
              placeholder=""
            />
          </div>
        </n-form-item>
        <n-form-item>
          <n-button type="success" tertiary @click="login" style="width: 100%;" size="large">登 录</n-button>
        </n-form-item>
      </template>
    </n-form>
  </n-card>
</template>

<script setup lang="ts">
import { reactive, ref, nextTick } from 'vue';
import { useMessage } from 'naive-ui';

const message = useMessage();
const form = reactive({
  phone: '',
  verificationCode: Array(6).fill(''),
});
const codeSent = ref(false);
const codeInputRefs = ref<any[]>([]);

const setCodeInputRef = (el: any, index: number) => {
  if (el) {
    codeInputRefs.value[index] = el;
  }
};

const getVerificationCode = () => {
  if (!/^1\d{10}$/.test(form.phone)) {
    message.error('请输入有效的手机号码');
    return;
  }
  console.log(`Sending verification code to ${form.phone}`);
  codeSent.value = true;
  message.success('验证码已发送');
  nextTick(() => {
    handleFocus(0);
  });
};

const handleCodeInput = (index: number) => {
  const value = form.verificationCode[index];
  if (value.match(/^\d$/) && index < 5) {
    codeInputRefs.value[index + 1]?.focus();
  }
};

const handleKeyDown = (index: number, event: KeyboardEvent) => {
  if (event.key === 'Backspace' && form.verificationCode[index] === '' && index > 0) {
    codeInputRefs.value[index - 1]?.focus();
  }
};

const handleFocus = (index: number) => {
  const firstEmptyIndex = form.verificationCode.findIndex(code => code === '');
  if (firstEmptyIndex !== -1 && index !== firstEmptyIndex) {
     codeInputRefs.value[firstEmptyIndex]?.focus();
  }
};

const login = () => {
  const code = form.verificationCode.join('');
  if (code.length !== 6 || !/^\d{6}$/.test(code)) {
    message.error('请输入完整的6位验证码');
    return;
  }
  console.log(`Logging in with phone: ${form.phone} and code: ${code}`);
  message.success('登录成功！');
  codeSent.value = false;
  form.phone = '';
  form.verificationCode.fill('');
};
</script>

<style scoped>
.box-card {
  max-width: 400px;
  margin: 40px auto;
}
.login-form {
  display: flex;
  flex-direction: column;
  gap: 10px;
}
.code-prompt {
  width: 100%;
  text-align: center;
  margin-bottom: 15px;
}
.code-inputs {
  display: flex;
  justify-content: space-between;
  gap: 10px;
  width: 100%;
}
.code-input {
  text-align: center;
}
.code-input :deep(input) {
  text-align: center;
  font-weight: bold;
}
</style>
