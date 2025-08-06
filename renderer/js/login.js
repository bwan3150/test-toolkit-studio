const { ipcRenderer } = require('electron');

document.addEventListener('DOMContentLoaded', () => {
    const loginForm = document.getElementById('loginForm');
    const emailInput = document.getElementById('email');
    const passwordInput = document.getElementById('password');
    const baseUrlInput = document.getElementById('baseUrl');
    const errorMessage = document.getElementById('errorMessage');
    const loginButton = document.querySelector('.login-button');
    const buttonText = document.getElementById('buttonText');
    const spinner = document.getElementById('spinner');

    // Load saved base URL if exists
    ipcRenderer.invoke('store-get', 'base_url').then(url => {
        if (url) {
            baseUrlInput.value = url;
        }
    });

    // Load saved email if exists
    ipcRenderer.invoke('store-get', 'last_email').then(email => {
        if (email) {
            emailInput.value = email;
        }
    });

    loginForm.addEventListener('submit', async (e) => {
        e.preventDefault();
        
        // Clear any previous error
        errorMessage.classList.remove('active');
        errorMessage.textContent = '';
        
        // Show loading state
        loginButton.classList.add('loading');
        buttonText.style.display = 'none';
        spinner.classList.add('active');
        
        const credentials = {
            email: emailInput.value,
            password: passwordInput.value,
            baseUrl: baseUrlInput.value
        };
        
        try {
            const result = await ipcRenderer.invoke('login', credentials);
            
            if (result.success) {
                // Save email for next time
                await ipcRenderer.invoke('store-set', 'last_email', credentials.email);
                
                // Navigate to main app
                await ipcRenderer.invoke('navigate-to-app');
            } else {
                // Show error
                errorMessage.textContent = result.error || 'Login failed. Please check your credentials.';
                errorMessage.classList.add('active');
                
                // Reset button state
                loginButton.classList.remove('loading');
                buttonText.style.display = 'inline';
                spinner.classList.remove('active');
            }
        } catch (error) {
            // Show error
            errorMessage.textContent = 'An unexpected error occurred. Please try again.';
            errorMessage.classList.add('active');
            
            // Reset button state
            loginButton.classList.remove('loading');
            buttonText.style.display = 'inline';
            spinner.classList.remove('active');
        }
    });

    // Handle Enter key in password field
    passwordInput.addEventListener('keypress', (e) => {
        if (e.key === 'Enter') {
            loginForm.dispatchEvent(new Event('submit'));
        }
    });

    // Clear error message when user starts typing
    [emailInput, passwordInput, baseUrlInput].forEach(input => {
        input.addEventListener('input', () => {
            if (errorMessage.classList.contains('active')) {
                errorMessage.classList.remove('active');
            }
        });
    });
});
