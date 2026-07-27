// Cart functionality with enhanced features
document.addEventListener('DOMContentLoaded', () => {
    // Initialize cart
    const cart = new ShoppingCart();
    cart.init();
});

class ShoppingCart {
    constructor() {
        this.cartIcon = document.querySelector('#cart-icon');
        this.cart = document.querySelector('.cart');
        this.closeCart = document.querySelector('#close-cart');
        this.cartContent = document.querySelector('.cart-content');
        this.totalPriceElement = document.querySelector('.total-price');
        this.cartCountElement = document.querySelector('.cart-count');
        this.themeToggle = document.querySelector('#theme-toggle');
        this.searchInput = document.querySelector('#search-input');
        this.searchIcon = document.querySelector('#search-icon');
        this.filterButtons = document.querySelectorAll('.filter-btn');
        this.products = document.querySelectorAll('.product-box');
        this.emptyCartMessage = document.querySelector('.empty-cart-message');
        this.continueShoppingBtn = document.querySelector('.continue-shopping');
        this.checkoutBtn = document.querySelector('.btn-checkout');
        this.clearCartBtn = document.querySelector('.btn-clear-cart');
        this.toast = document.querySelector('#toast');
        this.toastMessage = document.querySelector('#toast-message');
        this.cartItems = [];
    }

    init() {
        this.loadCartFromStorage();
        this.addEventListeners();
        this.updateCartDisplay();
        this.checkEmptyCart();
        this.loadTheme();
    }

    addEventListeners() {
        // Open cart
        this.cartIcon.addEventListener('click', () => this.openCart());

        // Close cart
        this.closeCart.addEventListener('click', () => this.closeCartFunc());

        // Theme toggle
        this.themeToggle.addEventListener('click', () => this.toggleTheme());

        // Search functionality
        this.searchIcon.addEventListener('click', () => this.searchProducts());
        this.searchInput.addEventListener('keyup', (e) => {
            if (e.key === 'Enter') this.searchProducts();
        });

        // Filter buttons
        this.filterButtons.forEach(btn => {
            btn.addEventListener('click', (e) => this.filterProducts(e));
        });

        // Add to cart buttons
        document.querySelectorAll('.add-cart').forEach(button => {
            button.addEventListener('click', (e) => this.addToCart(e));
        });

        // Continue shopping
        if (this.continueShoppingBtn) {
            this.continueShoppingBtn.addEventListener('click', () => this.closeCartFunc());
        }

        // Checkout button
        if (this.checkoutBtn) {
            this.checkoutBtn.addEventListener('click', () => this.checkout());
        }

        // Clear cart button
        if (this.clearCartBtn) {
            this.clearCartBtn.addEventListener('click', () => this.clearCart());
        }

        // Shop now button
        const shopNowBtn = document.querySelector('.shop-now-btn');
        if (shopNowBtn) {
            shopNowBtn.addEventListener('click', () => {
                document.querySelector('.shop').scrollIntoView({ behavior: 'smooth' });
            });
        }
    }

    openCart() {
        this.cart.classList.add('active');
        this.checkEmptyCart();
    }

    closeCartFunc() {
        this.cart.classList.remove('active');
    }

    toggleTheme() {
        document.body.classList.toggle('dark-mode');
        const isDarkMode = document.body.classList.contains('dark-mode');
        localStorage.setItem('theme', isDarkMode ? 'dark' : 'light');
        
        // Update icon
        this.themeToggle.classList.toggle('bx-moon');
        this.themeToggle.classList.toggle('bx-sun');
    }

    loadTheme() {
        const theme = localStorage.getItem('theme');
        if (theme === 'dark') {
            document.body.classList.add('dark-mode');
            this.themeToggle.classList.remove('bx-moon');
            this.themeToggle.classList.add('bx-sun');
        }
    }

    searchProducts() {
        const searchTerm = this.searchInput.value.toLowerCase().trim();
        
        this.products.forEach(product => {
            const title = product.querySelector('.product-title').textContent.toLowerCase();
            const category = product.dataset.category;
            
            if (title.includes(searchTerm) || category.includes(searchTerm)) {
                product.style.display = 'block';
                setTimeout(() => {
                    product.style.opacity = '1';
                    product.style.transform = 'scale(1)';
                }, 50);
            } else {
                product.style.opacity = '0';
                product.style.transform = 'scale(0.8)';
                setTimeout(() => {
                    product.style.display = 'none';
                }, 300);
            }
        });
    }

    filterProducts(event) {
        const filterBtn = event.target;
        const filterValue = filterBtn.dataset.filter;

        // Update active button
        this.filterButtons.forEach(btn => btn.classList.remove('active'));
        filterBtn.classList.add('active');

        // Filter products
        this.products.forEach(product => {
            const category = product.dataset.category;
            
            if (filterValue === 'all' || category === filterValue) {
                product.style.display = 'block';
                setTimeout(() => {
                    product.style.opacity = '1';
                    product.style.transform = 'scale(1)';
                }, 50);
            } else {
                product.style.opacity = '0';
                product.style.transform = 'scale(0.8)';
                setTimeout(() => {
                    product.style.display = 'none';
                }, 300);
            }
        });
    }

    addToCart(event) {
        const button = event.target;
        const productBox = button.closest('.product-box');
        const title = productBox.querySelector('.product-title').textContent;
        const price = parseFloat(productBox.querySelector('.price').textContent.replace('$', ''));
        const image = productBox.querySelector('.product-img').src;
        const category = productBox.dataset.category;

        // Check if item already in cart
        const existingItem = this.cartItems.find(item => item.title === title);
        
        if (existingItem) {
            existingItem.quantity += 1;
            this.showToast(`Increased quantity of ${title}`);
        } else {
            this.cartItems.push({
                title,
                price,
                image,
                quantity: 1,
                category
            });
            this.showToast(`${title} added to cart!`);
        }

        this.saveCartToStorage();
        this.updateCartDisplay();
        this.openCart();
    }

    removeFromCart(index) {
        this.cartItems.splice(index, 1);
        this.saveCartToStorage();
        this.updateCartDisplay();
        this.showToast('Item removed from cart');
    }

    updateQuantity(index, newQuantity) {
        if (newQuantity < 1) {
            this.removeFromCart(index);
            return;
        }
        
        this.cartItems[index].quantity = parseInt(newQuantity);
        this.saveCartToStorage();
        this.updateCartDisplay();
    }

    updateCartDisplay() {
        // Clear cart content
        this.cartContent.innerHTML = '';

        // Render cart items
        this.cartItems.forEach((item, index) => {
            const cartBox = document.createElement('div');
            cartBox.classList.add('cart-box');
            cartBox.innerHTML = `
                <img src="${item.image}" alt="${item.title}" class="cart-img">
                <div class="detail-box">
                    <div class="cart-product-title">${item.title}</div>
                    <div class="cart-price">$${item.price.toFixed(2)}</div>
                    <input type="number" value="${item.quantity}" min="1" class="cart-quantity" data-index="${index}">
                </div>
                <i class='bx bx-trash cart-remove' data-index="${index}"></i>
            `;

            this.cartContent.appendChild(cartBox);

            // Add event listeners for remove and quantity change
            cartBox.querySelector('.cart-remove').addEventListener('click', (e) => {
                this.removeFromCart(parseInt(e.target.dataset.index));
            });

            cartBox.querySelector('.cart-quantity').addEventListener('change', (e) => {
                this.updateQuantity(parseInt(e.target.dataset.index), e.target.value);
            });
        });

        // Update total
        this.calculateTotal();
        
        // Update cart count
        this.updateCartCount();
        
        // Check if cart is empty
        this.checkEmptyCart();
    }

    calculateTotal() {
        const total = this.cartItems.reduce((sum, item) => sum + (item.price * item.quantity), 0);
        this.totalPriceElement.textContent = `$${total.toFixed(2)}`;
    }

    updateCartCount() {
        const totalCount = this.cartItems.reduce((sum, item) => sum + item.quantity, 0);
        this.cartCountElement.textContent = totalCount;
        
        if (totalCount > 0) {
            this.cartCountElement.style.display = 'block';
        } else {
            this.cartCountElement.style.display = 'none';
        }
    }

    checkEmptyCart() {
        if (this.cartItems.length === 0) {
            this.emptyCartMessage.classList.add('show');
            this.cartContent.style.display = 'none';
        } else {
            this.emptyCartMessage.classList.remove('show');
            this.cartContent.style.display = 'block';
        }
    }

    clearCart() {
        if (confirm('Are you sure you want to clear your cart?')) {
            this.cartItems = [];
            this.saveCartToStorage();
            this.updateCartDisplay();
            this.showToast('Cart cleared!');
        }
    }

    checkout() {
        if (this.cartItems.length === 0) {
            this.showToast('Your cart is empty!');
            return;
        }

        const total = this.cartItems.reduce((sum, item) => sum + (item.price * item.quantity), 0);
        const itemCount = this.cartItems.reduce((sum, item) => sum + item.quantity, 0);
        
        alert(`Thank you for your order!\n\nItems: ${itemCount}\nTotal: $${total.toFixed(2)}\n\nYour order will be processed shortly.`);
        
        // Clear cart after checkout
        this.cartItems = [];
        this.saveCartToStorage();
        this.updateCartDisplay();
        this.closeCartFunc();
    }

    showToast(message) {
        this.toastMessage.textContent = message;
        this.toast.classList.add('show');
        
        setTimeout(() => {
            this.toast.classList.remove('show');
        }, 3000);
    }

    saveCartToStorage() {
        localStorage.setItem('cart', JSON.stringify(this.cartItems));
    }

    loadCartFromStorage() {
        const savedCart = localStorage.getItem('cart');
        if (savedCart) {
            this.cartItems = JSON.parse(savedCart);
        }
    }
}

