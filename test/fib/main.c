int fib(int n) {
    int prev = 1;
    int prev2 = 0;
    int curr = 1;
    for (int i = 0; i < n; ++i) {
        curr = prev + prev2;
        prev = prev2;
        prev2 = curr;
    }
    return curr;
}

double fdiv(double a, double b) {
    return a / b;
}

double main() {
    return fdiv(400, 3);
}
