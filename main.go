package main

// odr-webapi - a simple API for updating DLS text files
// This API receives text via POST requests and writes it to a file
// Version: 0.0.2

import (
	"flag"
	"fmt"
	"io"
	"log"
	"net/http"
	"os"
	"path/filepath"
	"time"
)

// Configuration for the application
type Config struct {
	Port       int
	TargetPath string
	AuthToken  string
}

// Function to get the last update time of the file
func getLastUpdateTime(filePath string) string {
	fileInfo, err := os.Stat(filePath)
	if err != nil {
		if os.IsNotExist(err) {
			return "Never"
		}
		log.Printf("Error getting file info: %v", err)
		return "Unknown"
	}
	return fileInfo.ModTime().Format(time.RFC3339)
}

// Middleware to check for a valid authentication token
func tokenAuthMiddleware(next http.HandlerFunc, config *Config) http.HandlerFunc {
	return func(w http.ResponseWriter, r *http.Request) {
		// Token is required for POST requests to /api/dls
		if r.URL.Path == "/api/dls" && r.Method == http.MethodPost {
			// Check the Authorization header
			token := r.Header.Get("Authorization")

			// If no token is provided or it doesn't match, return 401 Unauthorized
			if token != config.AuthToken {
				http.Error(w, "Unauthorized: Invalid or missing token", http.StatusUnauthorized)
				log.Printf("Unauthorized access attempt from %s", r.RemoteAddr)
				return
			}
		}

		// If token is valid or not required, proceed to the next handler
		next(w, r)
	}
}

func main() {
	// Configure command line options
	config := Config{}
	// Define flags without defaults - all are mandatory
	var port = flag.Int("port", 0, "Port on which the API should run (mandatory)")
	var target = flag.String("target", "", "Full path to DLS text file (mandatory)")
	var token = flag.String("token", "", "Authentication token for API access (mandatory for POST requests)")

	// Define custom usage function
	flag.Usage = func() {
		fmt.Println("odr-webapi - a simple API for updating DLS text files")
		fmt.Println("\nUsage:")
		fmt.Println("  odr-webapi -port PORT -target TARGET_PATH -token AUTH_TOKEN")
		fmt.Println("\nOptions:")
		fmt.Println("  -port PORT")
		fmt.Println("        Port on which the API should run")
		fmt.Println("  -target TARGET_PATH")
		fmt.Println("        Full path to DLS text file")
		fmt.Println("  -token AUTH_TOKEN")
		fmt.Println("        Authentication token for API access (required for POST requests)")
		fmt.Println("\nEndpoints:")
		fmt.Println("  POST /api/dls")
		fmt.Println("        Updates the DLS text file with the request body")
		fmt.Println("        Requires Authorization header with token")
		fmt.Println("  GET  /api/status")
		fmt.Println("        Returns the API status and last update time")
		fmt.Println("\nExample:")
		fmt.Println("  odr-webapi -port 9000 -target /dabplus/dls/dls.txt -token mySecretToken")
		fmt.Println("\nAuthentication:")
		fmt.Println("  All POST requests must include the token in the Authorization header:")
		fmt.Println("  curl -X POST -H \"Authorization: mySecretToken\" --data \"New DLS text\" http://localhost:9000/api/dls")
	}

	flag.Parse()

	// Check if all required parameters are provided
	if *port <= 0 || *target == "" || *token == "" {
		fmt.Println("Error: All parameters (port, target, token) are mandatory")
		flag.Usage()
		os.Exit(1)
	}

	// Assign values to config
	config.Port = *port
	config.TargetPath = *target
	config.AuthToken = *token

	// Check if target directory exists
	targetDir := filepath.Dir(config.TargetPath)
	if _, err := os.Stat(targetDir); os.IsNotExist(err) {
		log.Fatalf("Target directory does not exist: %s", targetDir)
	}

	// Handler for writing text data
	http.HandleFunc("/api/dls", tokenAuthMiddleware(func(w http.ResponseWriter, r *http.Request) {
		// Only allow POST method
		if r.Method != http.MethodPost {
			http.Error(w, "Only POST method is supported", http.StatusMethodNotAllowed)
			return
		}

		// Read the request content
		body, err := io.ReadAll(r.Body)
		if err != nil {
			http.Error(w, "Error reading request body", http.StatusBadRequest)
			log.Printf("Error reading request body: %v", err)
			return
		}
		defer r.Body.Close()

		// Write the text to the target file
		err = os.WriteFile(config.TargetPath, body, 0644)
		if err != nil {
			http.Error(w, "Error writing to file", http.StatusInternalServerError)
			log.Printf("Error writing to %s: %v", config.TargetPath, err)
			return
		}

		// Get the updated time directly from the file
		lastUpdateTime := getLastUpdateTime(config.TargetPath)

		// Confirm success
		w.WriteHeader(http.StatusOK)
		w.Header().Set("Content-Type", "application/json")
		fmt.Fprintf(w, `{"status":"success","message":"Text successfully updated","lastUpdate":"%s"}`, lastUpdateTime)
		log.Printf("Text successfully written to %s at %s", config.TargetPath, lastUpdateTime)
	}, &config))

	// Status endpoint to check if the API is running (no authentication required)
	http.HandleFunc("/api/status", func(w http.ResponseWriter, r *http.Request) {
		lastUpdateTime := getLastUpdateTime(config.TargetPath)
		w.Header().Set("Content-Type", "application/json")
		fmt.Fprintf(w, `{"status":"online","target":"%s","lastUpdate":"%s"}`,
			config.TargetPath, lastUpdateTime)
	})

	// Start the server
	serverAddr := fmt.Sprintf(":%d", config.Port)
	log.Printf("ODR Web API starting on http://localhost%s", serverAddr)
	log.Printf("DLS text will be written to: %s", config.TargetPath)
	log.Fatal(http.ListenAndServe(serverAddr, nil))
}
