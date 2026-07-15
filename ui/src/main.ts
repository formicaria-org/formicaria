import './app.css';
import { mount } from 'svelte';
import App from './App.svelte';

// Apply the saved theme (or the OS preference) before mount so there's no flash.
const saved = localStorage.getItem('fm-theme');
const theme = saved ?? (matchMedia('(prefers-color-scheme: light)').matches ? 'light' : 'dark');
document.documentElement.dataset.theme = theme;

export default mount(App, { target: document.getElementById('app')! });
