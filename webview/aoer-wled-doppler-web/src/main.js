import { createApp } from 'vue'
import App from './App.vue'
import '@picocss/pico'
import * as Vue from 'vue/dist/vue.esm-bundler'
window.Vue = Vue
import PicoVue from '@ginger-tek/pico-vue'

const app = createApp(App)
      .use(PicoVue)
      .mount("#app");
